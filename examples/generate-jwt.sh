#!/bin/bash
# Generate JWT Token for Mizuchi Uploadr
# Usage: ./generate-jwt.sh [options]
#
# Options:
#   -s, --secret SECRET     JWT secret (default: from JWT_SECRET env or default value)
#   -u, --subject SUBJECT   Subject claim (default: "user@example.com")
#   -e, --expiry HOURS      Expiry in hours (default: 24)
#   -i, --issuer ISSUER     Issuer claim (optional)
#   -a, --audience AUD      Audience claim (optional)
#   -h, --help              Show this help message

set -e

# Default values
SECRET="${JWT_SECRET:-your-super-secret-jwt-key-change-in-production}"
SUBJECT="user@example.com"
EXPIRY_HOURS=24
ISSUER=""
AUDIENCE=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--secret)
            SECRET="$2"
            shift 2
            ;;
        -u|--subject)
            SUBJECT="$2"
            shift 2
            ;;
        -e|--expiry)
            EXPIRY_HOURS="$2"
            shift 2
            ;;
        -i|--issuer)
            ISSUER="$2"
            shift 2
            ;;
        -a|--audience)
            AUDIENCE="$2"
            shift 2
            ;;
        -h|--help)
            echo "Generate JWT Token for Mizuchi Uploadr"
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  -s, --secret SECRET     JWT secret (default: from JWT_SECRET env)"
            echo "  -u, --subject SUBJECT   Subject claim (default: user@example.com)"
            echo "  -e, --expiry HOURS      Expiry in hours (default: 24)"
            echo "  -i, --issuer ISSUER     Issuer claim (optional)"
            echo "  -a, --audience AUD      Audience claim (optional)"
            echo "  -h, --help              Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 -u admin@example.com -e 1"
            echo "  $0 --secret mysecret --subject testuser --expiry 48"
            echo "  JWT_SECRET=mysecret $0 -u user1"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Check for required tools
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}Error: $1 is required but not installed.${NC}"
        exit 1
    fi
}

check_tool openssl
check_tool base64
check_tool jq

# Base64URL encode function (differs from standard base64)
base64url_encode() {
    # Convert standard base64 to base64url (replace +/ with -_, remove padding)
    base64 | tr '+/' '-_' | tr -d '='
}

# Calculate timestamps
NOW=$(date +%s)
EXP=$((NOW + EXPIRY_HOURS * 3600))

# Build header (always HS256)
HEADER='{"alg":"HS256","typ":"JWT"}'

# Build payload with optional claims using jq for safe JSON construction
# This prevents injection attacks from special characters in input values
PAYLOAD=$(jq -c -n \
  --arg sub "$SUBJECT" \
  --argjson iat "$NOW" \
  --argjson exp "$EXP" \
  --arg iss "$ISSUER" \
  --arg aud "$AUDIENCE" \
  '{sub: $sub, iat: $iat, exp: $exp}
   | if $iss | length > 0 then .iss = $iss else . end
   | if $aud | length > 0 then .aud = $aud else . end')

# Encode header and payload
HEADER_B64=$(echo -n "$HEADER" | base64url_encode)
PAYLOAD_B64=$(echo -n "$PAYLOAD" | base64url_encode)

# Create signature input
SIGNATURE_INPUT="${HEADER_B64}.${PAYLOAD_B64}"

# Calculate HMAC-SHA256 signature
SIGNATURE=$(echo -n "$SIGNATURE_INPUT" | openssl dgst -sha256 -hmac "$SECRET" -binary | base64url_encode)

# Combine to form JWT
JWT="${SIGNATURE_INPUT}.${SIGNATURE}"

# Output
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}JWT Token Generated Successfully${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Token Details:${NC}"
echo -e "  Subject:  ${YELLOW}${SUBJECT}${NC}"
echo -e "  Issued:   ${YELLOW}$(date -r $NOW '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -d @$NOW '+%Y-%m-%d %H:%M:%S')${NC}"
echo -e "  Expires:  ${YELLOW}$(date -r $EXP '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -d @$EXP '+%Y-%m-%d %H:%M:%S')${NC}"
if [[ -n "$ISSUER" ]]; then
    echo -e "  Issuer:   ${YELLOW}${ISSUER}${NC}"
fi
if [[ -n "$AUDIENCE" ]]; then
    echo -e "  Audience: ${YELLOW}${AUDIENCE}${NC}"
fi
echo ""
echo -e "${BLUE}JWT Token:${NC}"
echo -e "${YELLOW}${JWT}${NC}"
echo ""
echo -e "${BLUE}Usage with curl:${NC}"
echo -e "  curl -H \"Authorization: Bearer ${JWT}\" http://localhost:8080/private/test.txt"
echo ""
echo -e "${BLUE}Usage with uploader CLI:${NC}"
echo -e "  ./uploader.py --token ${JWT} upload myfile.txt /private/myfile.txt"
echo ""

# Also output just the token for piping
if [[ -t 1 ]]; then
    # Interactive terminal - already printed above
    :
else
    # Piped output - just print the token
    echo "$JWT"
fi
