//! AWS SigV4 Authentication
//!
//! Validates AWS Signature Version 4 signed requests.
//!
//! # Example
//!
//! ```
//! use mizuchi_uploadr::auth::sigv4::SigV4Authenticator;
//!
//! let mut auth = SigV4Authenticator::new("s3", "us-east-1");
//! auth.add_credentials("AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
//! ```

use super::{AuthError, AuthRequest, AuthResult, Authenticator};
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

/// Maximum allowed time skew in seconds (AWS allows 15 minutes)
const MAX_TIME_SKEW_SECONDS: i64 = 15 * 60;

/// Parsed SigV4 Authorization header components
#[derive(Debug)]
struct SigV4AuthHeader {
    access_key: String,
    date: String,
    region: String,
    service: String,
    signed_headers: Vec<String>,
    signature: String,
}

impl SigV4AuthHeader {
    /// Parse an AWS4-HMAC-SHA256 Authorization header
    fn parse(header: &str) -> Result<Self, AuthError> {
        // Expected format:
        // AWS4-HMAC-SHA256 Credential=ACCESS_KEY/DATE/REGION/SERVICE/aws4_request,
        // SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=SIGNATURE

        if !header.starts_with("AWS4-HMAC-SHA256 ") {
            return Err(AuthError::InvalidToken(
                "Invalid SigV4 authorization header".into(),
            ));
        }

        let parts: HashMap<String, String> = header
            .strip_prefix("AWS4-HMAC-SHA256 ")
            .unwrap()
            .split(", ")
            .filter_map(|part| {
                let mut iter = part.splitn(2, '=');
                let key = iter.next()?.trim();
                let value = iter.next()?.trim();
                Some((key.to_string(), value.to_string()))
            })
            .collect();

        // Parse Credential
        let credential = parts
            .get("Credential")
            .ok_or_else(|| AuthError::InvalidToken("Missing Credential".into()))?;

        let cred_parts: Vec<&str> = credential.split('/').collect();
        if cred_parts.len() != 5 || cred_parts[4] != "aws4_request" {
            return Err(AuthError::InvalidToken("Invalid Credential format".into()));
        }

        let access_key = cred_parts[0].to_string();
        let date = cred_parts[1].to_string();
        let region = cred_parts[2].to_string();
        let service = cred_parts[3].to_string();

        // Parse SignedHeaders
        let signed_headers_str = parts
            .get("SignedHeaders")
            .ok_or_else(|| AuthError::InvalidToken("Missing SignedHeaders".into()))?;
        let signed_headers: Vec<String> = signed_headers_str
            .split(';')
            .map(|s| s.to_string())
            .collect();

        // Parse Signature
        let signature = parts
            .get("Signature")
            .ok_or_else(|| AuthError::InvalidToken("Missing Signature".into()))?
            .to_string();

        Ok(Self {
            access_key,
            date,
            region,
            service,
            signed_headers,
            signature,
        })
    }

    /// Get the credential scope string
    fn credential_scope(&self) -> String {
        format!(
            "{}/{}/{}/aws4_request",
            self.date, self.region, self.service
        )
    }
}

/// SigV4 Authenticator
///
/// Validates AWS Signature Version 4 signed requests.
pub struct SigV4Authenticator {
    /// Expected service name (e.g., "s3"). If set, requests must use this service in credential scope.
    expected_service: Option<String>,
    /// Expected region (e.g., "us-east-1"). If set, requests must use this region in credential scope.
    expected_region: Option<String>,
    /// Mapping of access key ID to secret access key
    credentials_store: HashMap<String, String>,
}

impl SigV4Authenticator {
    /// Create a new SigV4 authenticator with expected service and region validation
    ///
    /// Requests must include matching service and region in their credential scope.
    pub fn new(service: &str, region: &str) -> Self {
        Self {
            expected_service: Some(service.to_string()),
            expected_region: Some(region.to_string()),
            credentials_store: HashMap::new(),
        }
    }

    /// Create a SigV4 authenticator that accepts any service/region
    ///
    /// Use this when you want to validate signatures without restricting scope.
    pub fn permissive() -> Self {
        Self {
            expected_service: None,
            expected_region: None,
            credentials_store: HashMap::new(),
        }
    }

    /// Validate that the request's credential scope matches expected values
    fn validate_scope(&self, parsed: &SigV4AuthHeader) -> Result<(), AuthError> {
        if let Some(expected) = &self.expected_service {
            if &parsed.service != expected {
                return Err(AuthError::InvalidToken(format!(
                    "Invalid service: expected '{}', got '{}'",
                    expected, parsed.service
                )));
            }
        }
        if let Some(expected) = &self.expected_region {
            if &parsed.region != expected {
                return Err(AuthError::InvalidToken(format!(
                    "Invalid region: expected '{}', got '{}'",
                    expected, parsed.region
                )));
            }
        }
        Ok(())
    }

    /// Add credentials for an access key
    pub fn add_credentials(&mut self, access_key: &str, secret_key: &str) {
        self.credentials_store
            .insert(access_key.to_string(), secret_key.to_string());
    }

    /// Compute HMAC-SHA256
    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// Compute SHA256 hash and return hex string
    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Derive the signing key
    fn derive_signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
        let k_secret = format!("AWS4{}", secret_key);
        let k_date = Self::hmac_sha256(k_secret.as_bytes(), date.as_bytes());
        let k_region = Self::hmac_sha256(&k_date, region.as_bytes());
        let k_service = Self::hmac_sha256(&k_region, service.as_bytes());
        Self::hmac_sha256(&k_service, b"aws4_request")
    }

    /// Build the canonical request string
    fn build_canonical_request(
        request: &AuthRequest,
        signed_headers: &[String],
    ) -> Result<String, AuthError> {
        // Method
        let method = &request.method;

        // Canonical URI (path)
        let canonical_uri = &request.path;

        // Canonical query string (empty for now, could be extended)
        let canonical_query = request.query.as_deref().unwrap_or("");

        // Canonical headers (sorted, lowercase)
        let mut canonical_headers = String::new();
        for header in signed_headers {
            let value = request
                .headers
                .get(header)
                .ok_or_else(|| {
                    AuthError::InvalidToken(format!("Missing signed header: {}", header))
                })?;
            canonical_headers.push_str(&format!("{}:{}\n", header, value.trim()));
        }

        // Signed headers string
        let signed_headers_str = signed_headers.join(";");

        // Payload hash (x-amz-content-sha256)
        let payload_hash = request
            .headers
            .get("x-amz-content-sha256")
            .map(|s| s.as_str())
            .unwrap_or("UNSIGNED-PAYLOAD");

        Ok(format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, canonical_uri, canonical_query, canonical_headers, signed_headers_str, payload_hash
        ))
    }

    /// Build the string to sign
    fn build_string_to_sign(
        timestamp: &str,
        credential_scope: &str,
        canonical_request_hash: &str,
    ) -> String {
        format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, canonical_request_hash
        )
    }

    /// Validate the request timestamp
    fn validate_timestamp(timestamp: &str) -> Result<(), AuthError> {
        // Parse timestamp (format: YYYYMMDDTHHMMSSZ)
        let parsed = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H%M%SZ")
            .map_err(|_| AuthError::InvalidToken("Invalid timestamp format".into()))?;

        let request_time = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            parsed,
            chrono::Utc,
        );
        let now = chrono::Utc::now();
        let diff = (now - request_time).num_seconds().abs();

        if diff > MAX_TIME_SKEW_SECONDS {
            return Err(AuthError::InvalidToken(format!(
                "Request timestamp too far from current time: {} seconds",
                diff
            )));
        }

        Ok(())
    }

    /// Constant-time comparison of signatures
    fn constant_time_compare(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.bytes().zip(b.bytes()) {
            result |= x ^ y;
        }
        result == 0
    }
}

#[async_trait]
impl Authenticator for SigV4Authenticator {
    #[cfg_attr(feature = "tracing", tracing::instrument(
        name = "auth.sigv4",
        skip(self, request),
        fields(
            auth.method = "sigv4",
            auth.signature_present = %request.headers.contains_key("authorization"),
            otel.kind = "internal"
        ),
        err
    ))]
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, AuthError> {
        // Step 1: Get and parse Authorization header
        let auth_header = request
            .headers
            .get("authorization")
            .ok_or(AuthError::MissingAuth)?;

        let parsed = SigV4AuthHeader::parse(auth_header)?;

        // Step 2: Validate credential scope (service/region)
        self.validate_scope(&parsed)?;

        // Step 3: Validate timestamp
        let timestamp = request
            .headers
            .get("x-amz-date")
            .ok_or_else(|| AuthError::InvalidToken("Missing x-amz-date header".into()))?;

        Self::validate_timestamp(timestamp)?;

        // Step 4: Look up secret key
        let secret_key = self
            .credentials_store
            .get(&parsed.access_key)
            .ok_or_else(|| {
                AuthError::InvalidToken(format!("Unknown access key: {}", parsed.access_key))
            })?;

        // Step 5: Build canonical request
        let canonical_request =
            Self::build_canonical_request(request, &parsed.signed_headers)?;
        let canonical_request_hash = Self::sha256_hex(canonical_request.as_bytes());

        // Step 6: Build string to sign
        let string_to_sign = Self::build_string_to_sign(
            timestamp,
            &parsed.credential_scope(),
            &canonical_request_hash,
        );

        // Step 7: Derive signing key and compute signature
        let signing_key = Self::derive_signing_key(
            secret_key,
            &parsed.date,
            &parsed.region,
            &parsed.service,
        );
        let computed_signature = hex::encode(Self::hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        // Step 8: Compare signatures (constant-time)
        if !Self::constant_time_compare(&computed_signature, &parsed.signature) {
            return Err(AuthError::InvalidSignature);
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            access_key = %parsed.access_key,
            "SigV4 authentication successful"
        );

        Ok(AuthResult {
            subject: parsed.access_key,
            claims: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigv4_authenticator_creation() {
        let auth = SigV4Authenticator::new("s3", "us-east-1");
        assert_eq!(auth.expected_service, Some("s3".to_string()));
        assert_eq!(auth.expected_region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_permissive_authenticator() {
        let auth = SigV4Authenticator::permissive();
        assert!(auth.expected_service.is_none());
        assert!(auth.expected_region.is_none());
    }

    #[tokio::test]
    async fn test_missing_auth_header() {
        let auth = SigV4Authenticator::new("s3", "us-east-1");
        let request = AuthRequest {
            headers: HashMap::new(),
            query: None,
            method: "PUT".into(),
            path: "/bucket/key".into(),
        };

        let result = auth.authenticate(&request).await;
        assert!(matches!(result, Err(AuthError::MissingAuth)));
    }

    #[test]
    fn test_parse_auth_header() {
        let header = "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=abcd1234";
        let parsed = SigV4AuthHeader::parse(header).unwrap();

        assert_eq!(parsed.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(parsed.date, "20230101");
        assert_eq!(parsed.region, "us-east-1");
        assert_eq!(parsed.service, "s3");
        assert_eq!(
            parsed.signed_headers,
            vec!["host", "x-amz-content-sha256", "x-amz-date"]
        );
        assert_eq!(parsed.signature, "abcd1234");
    }

    #[test]
    fn test_derive_signing_key() {
        // Test vector from AWS documentation
        let key = SigV4Authenticator::derive_signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        // The key should be 32 bytes (256 bits)
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(SigV4Authenticator::constant_time_compare("abc", "abc"));
        assert!(!SigV4Authenticator::constant_time_compare("abc", "abd"));
        assert!(!SigV4Authenticator::constant_time_compare("abc", "ab"));
        assert!(!SigV4Authenticator::constant_time_compare("ab", "abc"));
    }
}
