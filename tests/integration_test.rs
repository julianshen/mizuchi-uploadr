//! Integration tests for Mizuchi Uploadr

use mizuchi_uploadr::router::{S3Operation, S3RequestParser};

#[test]
fn test_parse_put_object_integration() {
    let op = S3RequestParser::parse("PUT", "/my-bucket/path/to/file.txt", None).unwrap();
    
    match op {
        S3Operation::PutObject { bucket, key } => {
            assert_eq!(bucket, "my-bucket");
            assert_eq!(key, "path/to/file.txt");
        }
        _ => panic!("Expected PutObject operation"),
    }
}

#[test]
fn test_parse_multipart_upload_integration() {
    let op = S3RequestParser::parse("POST", "/my-bucket/large-file.bin", Some("uploads")).unwrap();
    
    match op {
        S3Operation::CreateMultipartUpload { bucket, key } => {
            assert_eq!(bucket, "my-bucket");
            assert_eq!(key, "large-file.bin");
        }
        _ => panic!("Expected CreateMultipartUpload operation"),
    }
}

#[test]
fn test_reject_get_operation() {
    let result = S3RequestParser::parse("GET", "/my-bucket/file.txt", None);
    assert!(result.is_err());
}

#[test]
fn test_zero_copy_availability() {
    let available = mizuchi_uploadr::zero_copy_available();
    
    #[cfg(target_os = "linux")]
    assert!(available, "Zero-copy should be available on Linux");
    
    #[cfg(not(target_os = "linux"))]
    assert!(!available, "Zero-copy should not be available on non-Linux");
}
