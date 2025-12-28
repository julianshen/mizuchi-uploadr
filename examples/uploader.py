#!/usr/bin/env python3
"""
Mizuchi Uploadr CLI Client

A command-line tool for uploading files to Mizuchi Uploadr with support for:
- Simple uploads for small files
- Parallel multipart uploads for large files
- JWT authentication
- Progress display

Usage:
    ./uploader.py upload <file> <destination> [options]
    ./uploader.py --help

Examples:
    # Upload a small file (simple upload)
    ./uploader.py upload photo.jpg /uploads/photo.jpg

    # Upload a large file with parallel chunks
    ./uploader.py upload video.mp4 /private/video.mp4 --token <jwt>

    # Upload with custom settings
    ./uploader.py upload backup.tar.gz /private/backup.tar.gz \
        --token <jwt> \
        --chunk-size 20M \
        --parallel 8 \
        --endpoint http://localhost:8080
"""

import argparse
import hashlib
import os
import sys
import time
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from threading import Lock
from typing import List, Optional, Tuple
from urllib.parse import urljoin, quote

try:
    import requests
except ImportError:
    print("Error: 'requests' library is required.")
    print("Install with: pip install requests")
    sys.exit(1)


# Default configuration
DEFAULT_ENDPOINT = "http://localhost:8080"
DEFAULT_CHUNK_SIZE = 10 * 1024 * 1024  # 10MB
DEFAULT_MULTIPART_THRESHOLD = 50 * 1024 * 1024  # 50MB
DEFAULT_PARALLEL = 4


def parse_size(size_str: str) -> int:
    """Parse human-readable size string to bytes."""
    size_str = size_str.strip().upper()
    units = {
        'B': 1,
        'K': 1024,
        'KB': 1024,
        'M': 1024 * 1024,
        'MB': 1024 * 1024,
        'G': 1024 * 1024 * 1024,
        'GB': 1024 * 1024 * 1024,
    }

    for unit, multiplier in sorted(units.items(), key=lambda x: -len(x[0])):
        if size_str.endswith(unit):
            try:
                return int(float(size_str[:-len(unit)]) * multiplier)
            except ValueError:
                pass

    try:
        return int(size_str)
    except ValueError:
        raise ValueError(f"Invalid size format: {size_str}")


def format_size(size: int) -> str:
    """Format bytes to human-readable string."""
    for unit in ['B', 'KB', 'MB', 'GB', 'TB']:
        if size < 1024:
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} PB"


def format_time(seconds: float) -> str:
    """Format seconds to human-readable time."""
    if seconds < 60:
        return f"{seconds:.1f}s"
    elif seconds < 3600:
        return f"{int(seconds // 60)}m {int(seconds % 60)}s"
    else:
        return f"{int(seconds // 3600)}h {int((seconds % 3600) // 60)}m"


@dataclass
class UploadProgress:
    """Track upload progress across parallel uploads."""
    total_size: int
    uploaded: int = 0
    lock: Lock = None
    start_time: float = None

    def __post_init__(self):
        self.lock = Lock()
        self.start_time = time.time()

    def add(self, size: int):
        with self.lock:
            self.uploaded += size

    def display(self):
        with self.lock:
            percentage = (self.uploaded / self.total_size) * 100 if self.total_size > 0 else 0
            elapsed = time.time() - self.start_time
            speed = self.uploaded / elapsed if elapsed > 0 else 0
            remaining = (self.total_size - self.uploaded) / speed if speed > 0 else 0

            bar_width = 40
            filled = int(bar_width * percentage / 100)
            bar = '█' * filled + '░' * (bar_width - filled)

            sys.stdout.write(f"\r[{bar}] {percentage:5.1f}% | "
                           f"{format_size(self.uploaded)}/{format_size(self.total_size)} | "
                           f"{format_size(speed)}/s | ETA: {format_time(remaining)}  ")
            sys.stdout.flush()


class MizuchiUploader:
    """Client for uploading files to Mizuchi Uploadr."""

    def __init__(
        self,
        endpoint: str = DEFAULT_ENDPOINT,
        token: Optional[str] = None,
        chunk_size: int = DEFAULT_CHUNK_SIZE,
        multipart_threshold: int = DEFAULT_MULTIPART_THRESHOLD,
        parallel: int = DEFAULT_PARALLEL,
        verbose: bool = False,
    ):
        self.endpoint = endpoint.rstrip('/')
        self.token = token
        self.chunk_size = chunk_size
        self.multipart_threshold = multipart_threshold
        self.parallel = parallel
        self.verbose = verbose
        self.session = requests.Session()

        # Ensure chunk size is at least 5MB (S3 minimum)
        if self.chunk_size < 5 * 1024 * 1024:
            print(f"Warning: Chunk size increased to 5MB (S3 minimum)")
            self.chunk_size = 5 * 1024 * 1024

    def _get_headers(self) -> dict:
        """Get request headers including auth if configured."""
        headers = {
            'User-Agent': 'mizuchi-uploader/1.0',
        }
        if self.token:
            headers['Authorization'] = f'Bearer {self.token}'
        return headers

    def _log(self, message: str):
        """Print verbose message."""
        if self.verbose:
            print(f"[DEBUG] {message}")

    def upload(self, file_path: str, destination: str) -> bool:
        """Upload a file to the destination path."""
        path = Path(file_path)

        if not path.exists():
            print(f"Error: File not found: {file_path}")
            return False

        if not path.is_file():
            print(f"Error: Not a file: {file_path}")
            return False

        file_size = path.stat().st_size

        print(f"Uploading: {path.name}")
        print(f"Size: {format_size(file_size)}")
        print(f"Destination: {destination}")
        print()

        if file_size > self.multipart_threshold:
            self._log(f"File exceeds {format_size(self.multipart_threshold)}, using multipart upload")
            return self._multipart_upload(path, destination, file_size)
        else:
            self._log("Using simple upload")
            return self._simple_upload(path, destination, file_size)

    def _simple_upload(self, path: Path, destination: str, file_size: int) -> bool:
        """Perform a simple PUT upload for small files."""
        url = f"{self.endpoint}{destination}"
        progress = UploadProgress(file_size)

        try:
            with open(path, 'rb') as f:
                # Wrap file for progress tracking
                def file_reader():
                    chunk_read_size = 8192
                    while True:
                        data = f.read(chunk_read_size)
                        if not data:
                            break
                        progress.add(len(data))
                        progress.display()
                        yield data

                response = self.session.put(
                    url,
                    data=file_reader(),
                    headers={**self._get_headers(), 'Content-Length': str(file_size)},
                )

            print()  # New line after progress bar

            if response.status_code in (200, 201):
                print(f"✓ Upload successful!")
                return True
            else:
                print(f"✗ Upload failed: {response.status_code}")
                print(f"  Response: {response.text[:500]}")
                return False

        except Exception as e:
            print()
            print(f"✗ Upload error: {e}")
            return False

    def _multipart_upload(self, path: Path, destination: str, file_size: int) -> bool:
        """Perform multipart upload for large files with parallel chunk uploads."""

        # Step 1: Initiate multipart upload
        upload_id = self._initiate_multipart(destination)
        if not upload_id:
            return False

        self._log(f"Multipart upload initiated: {upload_id}")

        # Step 2: Calculate parts
        parts = self._calculate_parts(file_size)
        print(f"Uploading in {len(parts)} parts ({format_size(self.chunk_size)} each)")
        print(f"Parallel uploads: {self.parallel}")
        print()

        progress = UploadProgress(file_size)
        completed_parts: List[Tuple[int, str]] = []
        parts_lock = Lock()

        # Step 3: Upload parts in parallel
        try:
            with ThreadPoolExecutor(max_workers=self.parallel) as executor:
                futures = {}

                for part_number, start, end in parts:
                    future = executor.submit(
                        self._upload_part,
                        path, destination, upload_id, part_number, start, end, progress
                    )
                    futures[future] = part_number

                for future in as_completed(futures):
                    part_number = futures[future]
                    try:
                        etag = future.result()
                        if etag:
                            with parts_lock:
                                completed_parts.append((part_number, etag))
                        else:
                            raise Exception(f"Part {part_number} upload failed")
                    except Exception as e:
                        print()
                        print(f"✗ Part {part_number} failed: {e}")
                        self._abort_multipart(destination, upload_id)
                        return False

        except KeyboardInterrupt:
            print()
            print("Upload interrupted, aborting...")
            self._abort_multipart(destination, upload_id)
            return False

        print()  # New line after progress bar

        # Step 4: Complete multipart upload
        completed_parts.sort(key=lambda x: x[0])
        success = self._complete_multipart(destination, upload_id, completed_parts)

        if success:
            elapsed = time.time() - progress.start_time
            speed = file_size / elapsed if elapsed > 0 else 0
            print(f"✓ Upload successful!")
            print(f"  Time: {format_time(elapsed)}")
            print(f"  Average speed: {format_size(speed)}/s")

        return success

    def _calculate_parts(self, file_size: int) -> List[Tuple[int, int, int]]:
        """Calculate part boundaries for multipart upload."""
        parts = []
        part_number = 1
        position = 0

        while position < file_size:
            start = position
            end = min(position + self.chunk_size, file_size)
            parts.append((part_number, start, end))
            part_number += 1
            position = end

        return parts

    def _initiate_multipart(self, destination: str) -> Optional[str]:
        """Initiate a multipart upload and return the upload ID."""
        url = f"{self.endpoint}{destination}?uploads"

        try:
            response = self.session.post(url, headers=self._get_headers())

            if response.status_code == 200:
                # Parse XML response to get upload ID
                root = ET.fromstring(response.text)
                # Handle S3 namespace
                ns = {'s3': 'http://s3.amazonaws.com/doc/2006-03-01/'}
                upload_id_elem = root.find('.//UploadId') or root.find('.//s3:UploadId', ns)

                if upload_id_elem is not None:
                    return upload_id_elem.text
                else:
                    print(f"Error: Could not parse upload ID from response")
                    self._log(f"Response: {response.text}")
                    return None
            else:
                print(f"Error: Failed to initiate multipart upload: {response.status_code}")
                print(f"  Response: {response.text[:500]}")
                return None

        except Exception as e:
            print(f"Error: Failed to initiate multipart upload: {e}")
            return None

    def _upload_part(
        self,
        path: Path,
        destination: str,
        upload_id: str,
        part_number: int,
        start: int,
        end: int,
        progress: UploadProgress,
    ) -> Optional[str]:
        """Upload a single part and return the ETag."""
        url = f"{self.endpoint}{destination}?partNumber={part_number}&uploadId={upload_id}"
        part_size = end - start

        self._log(f"Uploading part {part_number}: bytes {start}-{end} ({format_size(part_size)})")

        try:
            with open(path, 'rb') as f:
                f.seek(start)
                data = f.read(part_size)

            response = self.session.put(
                url,
                data=data,
                headers={**self._get_headers(), 'Content-Length': str(part_size)},
            )

            progress.add(part_size)
            progress.display()

            if response.status_code == 200:
                etag = response.headers.get('ETag', '').strip('"')
                self._log(f"Part {part_number} completed: ETag={etag}")
                return etag
            else:
                self._log(f"Part {part_number} failed: {response.status_code} - {response.text[:200]}")
                return None

        except Exception as e:
            self._log(f"Part {part_number} error: {e}")
            return None

    def _complete_multipart(
        self,
        destination: str,
        upload_id: str,
        parts: List[Tuple[int, str]],
    ) -> bool:
        """Complete the multipart upload."""
        url = f"{self.endpoint}{destination}?uploadId={upload_id}"

        # Build completion XML
        xml_parts = ''.join(
            f'<Part><PartNumber>{part_num}</PartNumber><ETag>"{etag}"</ETag></Part>'
            for part_num, etag in parts
        )
        body = f'<?xml version="1.0" encoding="UTF-8"?><CompleteMultipartUpload>{xml_parts}</CompleteMultipartUpload>'

        self._log(f"Completing multipart upload with {len(parts)} parts")

        try:
            response = self.session.post(
                url,
                data=body,
                headers={**self._get_headers(), 'Content-Type': 'application/xml'},
            )

            if response.status_code == 200:
                return True
            else:
                print(f"Error: Failed to complete multipart upload: {response.status_code}")
                print(f"  Response: {response.text[:500]}")
                return False

        except Exception as e:
            print(f"Error: Failed to complete multipart upload: {e}")
            return False

    def _abort_multipart(self, destination: str, upload_id: str):
        """Abort a multipart upload."""
        url = f"{self.endpoint}{destination}?uploadId={upload_id}"

        try:
            response = self.session.delete(url, headers=self._get_headers())
            if response.status_code == 204:
                self._log("Multipart upload aborted successfully")
            else:
                self._log(f"Failed to abort multipart upload: {response.status_code}")
        except Exception as e:
            self._log(f"Error aborting multipart upload: {e}")


def main():
    parser = argparse.ArgumentParser(
        description='Mizuchi Uploadr CLI Client',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Upload a small file
  %(prog)s upload photo.jpg /uploads/photo.jpg

  # Upload a large file with parallel chunks
  %(prog)s upload video.mp4 /private/video.mp4 --token <jwt>

  # Custom chunk size and parallelism
  %(prog)s upload backup.tar.gz /private/backup.tar.gz \\
      --chunk-size 20M --parallel 8

  # Upload to custom endpoint
  %(prog)s upload file.txt /uploads/file.txt \\
      --endpoint http://my-server:8080
        """
    )

    subparsers = parser.add_subparsers(dest='command', help='Commands')

    # Upload command
    upload_parser = subparsers.add_parser('upload', help='Upload a file')
    upload_parser.add_argument('file', help='Local file to upload')
    upload_parser.add_argument('destination', help='Destination path (e.g., /uploads/file.txt)')
    upload_parser.add_argument(
        '--endpoint', '-e',
        default=os.environ.get('MIZUCHI_ENDPOINT', DEFAULT_ENDPOINT),
        help=f'Server endpoint (default: {DEFAULT_ENDPOINT})',
    )
    upload_parser.add_argument(
        '--token', '-t',
        default=os.environ.get('MIZUCHI_TOKEN'),
        help='JWT token for authentication',
    )
    upload_parser.add_argument(
        '--chunk-size', '-c',
        default='10M',
        help='Chunk size for multipart upload (default: 10M)',
    )
    upload_parser.add_argument(
        '--threshold', '-T',
        default='50M',
        help='Multipart upload threshold (default: 50M)',
    )
    upload_parser.add_argument(
        '--parallel', '-p',
        type=int,
        default=DEFAULT_PARALLEL,
        help=f'Number of parallel uploads (default: {DEFAULT_PARALLEL})',
    )
    upload_parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Enable verbose output',
    )

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    if args.command == 'upload':
        try:
            chunk_size = parse_size(args.chunk_size)
            threshold = parse_size(args.threshold)
        except ValueError as e:
            print(f"Error: {e}")
            sys.exit(1)

        uploader = MizuchiUploader(
            endpoint=args.endpoint,
            token=args.token,
            chunk_size=chunk_size,
            multipart_threshold=threshold,
            parallel=args.parallel,
            verbose=args.verbose,
        )

        success = uploader.upload(args.file, args.destination)
        sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
