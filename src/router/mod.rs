//! S3 API Router
//!
//! Parses incoming requests and routes them to appropriate handlers.

use thiserror::Error;

/// Router errors
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Bucket not found: {0}")]
    BucketNotFound(String),
}

/// S3 operation types
#[derive(Debug, Clone, PartialEq)]
pub enum S3Operation {
    /// PUT /{bucket}/{key}
    PutObject {
        bucket: String,
        key: String,
    },
    /// POST /{bucket}/{key}?uploads
    CreateMultipartUpload {
        bucket: String,
        key: String,
    },
    /// PUT /{bucket}/{key}?partNumber=N&uploadId=X
    UploadPart {
        bucket: String,
        key: String,
        part_number: u32,
        upload_id: String,
    },
    /// POST /{bucket}/{key}?uploadId=X
    CompleteMultipartUpload {
        bucket: String,
        key: String,
        upload_id: String,
    },
    /// DELETE /{bucket}/{key}?uploadId=X
    AbortMultipartUpload {
        bucket: String,
        key: String,
        upload_id: String,
    },
    /// GET /{bucket}/{key}?uploadId=X
    ListParts {
        bucket: String,
        key: String,
        upload_id: String,
    },
}

/// S3 Request Parser
pub struct S3RequestParser;

impl S3RequestParser {
    /// Parse an HTTP request into an S3 operation
    pub fn parse(
        method: &str,
        path: &str,
        query: Option<&str>,
    ) -> Result<S3Operation, RouterError> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.splitn(2, '/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Err(RouterError::InvalidPath("Missing bucket".into()));
        }

        let bucket = parts[0].to_string();
        let key = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

        if key.is_empty() && method != "HEAD" {
            return Err(RouterError::InvalidPath("Missing key".into()));
        }

        let query_params = Self::parse_query(query);

        match method {
            "PUT" => {
                if let (Some(part_number), Some(upload_id)) = (
                    query_params.get("partNumber"),
                    query_params.get("uploadId"),
                ) {
                    Ok(S3Operation::UploadPart {
                        bucket,
                        key,
                        part_number: part_number.parse().unwrap_or(0),
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Ok(S3Operation::PutObject { bucket, key })
                }
            }
            "POST" => {
                if query_params.contains_key("uploads") {
                    Ok(S3Operation::CreateMultipartUpload { bucket, key })
                } else if let Some(upload_id) = query_params.get("uploadId") {
                    Ok(S3Operation::CompleteMultipartUpload {
                        bucket,
                        key,
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Err(RouterError::InvalidPath("Invalid POST operation".into()))
                }
            }
            "DELETE" => {
                if let Some(upload_id) = query_params.get("uploadId") {
                    Ok(S3Operation::AbortMultipartUpload {
                        bucket,
                        key,
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Err(RouterError::MethodNotAllowed(
                        "DELETE only allowed for multipart abort".into(),
                    ))
                }
            }
            "GET" => {
                if let Some(upload_id) = query_params.get("uploadId") {
                    Ok(S3Operation::ListParts {
                        bucket,
                        key,
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Err(RouterError::MethodNotAllowed(
                        "GET not allowed (upload-only proxy)".into(),
                    ))
                }
            }
            _ => Err(RouterError::MethodNotAllowed(format!(
                "Method {} not allowed",
                method
            ))),
        }
    }

    fn parse_query(query: Option<&str>) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        if let Some(q) = query {
            for pair in q.split('&') {
                let mut kv = pair.splitn(2, '=');
                if let Some(key) = kv.next() {
                    let value = kv.next().unwrap_or("");
                    params.insert(key.to_string(), value.to_string());
                }
            }
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_put_object() {
        let op = S3RequestParser::parse("PUT", "/bucket/key", None).unwrap();
        assert_eq!(
            op,
            S3Operation::PutObject {
                bucket: "bucket".into(),
                key: "key".into()
            }
        );
    }

    #[test]
    fn test_parse_create_multipart() {
        let op = S3RequestParser::parse("POST", "/bucket/key", Some("uploads")).unwrap();
        assert_eq!(
            op,
            S3Operation::CreateMultipartUpload {
                bucket: "bucket".into(),
                key: "key".into()
            }
        );
    }

    #[test]
    fn test_parse_upload_part() {
        let op =
            S3RequestParser::parse("PUT", "/bucket/key", Some("partNumber=1&uploadId=abc123"))
                .unwrap();
        assert_eq!(
            op,
            S3Operation::UploadPart {
                bucket: "bucket".into(),
                key: "key".into(),
                part_number: 1,
                upload_id: "abc123".into()
            }
        );
    }

    #[test]
    fn test_parse_get_not_allowed() {
        let result = S3RequestParser::parse("GET", "/bucket/key", None);
        assert!(result.is_err());
    }
}
