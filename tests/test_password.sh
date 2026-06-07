#!/bin/bash
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEST_DIR="$(dirname "$(realpath "$0")")"
PROJECT_ROOT="$(dirname "$TEST_DIR")"
TEST_FILES_DIR="$TEST_DIR/test_files"

mkdir -p "$TEST_FILES_DIR"
TEST_FILE="$TEST_FILES_DIR/password_protected.bin"
echo "This is secure content that requires a password to download." > "$TEST_FILE"

# Start the server on custom ports to avoid conflicts
echo -e "${BLUE}==> Starting server on HTTP port 8085 and UDP port 5005...${NC}"
./target/release/server --http-port 8085 --udp-port 5005 --db-path ./db/test_data.sqlite --upload-dir ./test_uploads &
SERVER_PID=$!

# Ensure cleanup on exit
cleanup() {
    echo -e "${YELLOW}==> Cleaning up background server (PID: $SERVER_PID)...${NC}"
    kill $SERVER_PID 2>/dev/null || true
    rm -rf ./test_uploads ./db/test_data.sqlite
    echo -e "${GREEN}==> Cleanup complete.${NC}"
}
trap cleanup EXIT

# Wait for server to bind
sleep 1.5

# Calculate Hash ID from file to match client output
# The client uses XXH3 hash and formats as unique id
echo -e "${BLUE}==> Uploading file with password 'mysecret123'...${NC}"
CLIENT_OUTPUT=$(./target/release/client_cli "$TEST_FILE" --http-port 8085 --udp-port 5005 --password "mysecret123")
echo "$CLIENT_OUTPUT"

# Extract Hash ID (packet code) from client output
HASH_ID=$(echo "$CLIENT_OUTPUT" | grep -oP 'Hash ID\): \K[a-zA-Z0-9_-]+')
if [ -z "$HASH_ID" ]; then
    echo -e "${RED}✘ Failed to parse Hash ID from client output!${NC}"
    exit 1
fi
echo -e "${GREEN}✔ Found file Hash ID: $HASH_ID${NC}"

# Test 1: Fetch list of uploads and verify password info
echo -e "${BLUE}==> Test 1: Fetching upload list...${NC}"
LIST_JSON=$(curl -s http://localhost:8085/api/list)
echo "Response: $LIST_JSON"

# Check if 'has_password' is true
if echo "$LIST_JSON" | grep -q '"has_password":true'; then
    echo -e "${GREEN}✔ Test 1 passed: 'has_password' is true in upload list.${NC}"
else
    echo -e "${RED}✘ Test 1 failed: 'has_password' is not true!${NC}"
    exit 1
fi

# Check that the actual password field is NOT leaked in the list
if echo "$LIST_JSON" | grep -q '"password"'; then
    echo -e "${RED}✘ Test 1 failed: password field is leaked in JSON response!${NC}"
    exit 1
else
    echo -e "${GREEN}✔ Test 1 passed: Password is not leaked in JSON response.${NC}"
fi

# Test 2: Verify password check endpoint
echo -e "${BLUE}==> Test 2: Checking verify password endpoint...${NC}"

# Correct password
CORRECT_CHECK=$(curl -s -X POST -H "Content-Type: application/json" -d "{\"packet_code\":\"$HASH_ID\", \"password\":\"mysecret123\"}" http://localhost:8085/api/verify_password)
echo "Correct password verify response: $CORRECT_CHECK"
if echo "$CORRECT_CHECK" | grep -q '"success":true'; then
    echo -e "${GREEN}✔ Test 2.1 passed: Correct password accepted.${NC}"
else
    echo -e "${RED}✘ Test 2.1 failed: Correct password rejected!${NC}"
    exit 1
fi

# Incorrect password
WRONG_CHECK=$(curl -s -X POST -H "Content-Type: application/json" -d "{\"packet_code\":\"$HASH_ID\", \"password\":\"wrongpassword\"}" http://localhost:8085/api/verify_password)
echo "Incorrect password verify response: $WRONG_CHECK"
if echo "$WRONG_CHECK" | grep -q '"success":false'; then
    echo -e "${GREEN}✔ Test 2.2 passed: Incorrect password rejected.${NC}"
else
    echo -e "${RED}✘ Test 2.2 failed: Incorrect password accepted!${NC}"
    exit 1
fi

# Test 3: Download attempts
echo -e "${BLUE}==> Test 3: Attempting file downloads...${NC}"

# Download without password query parameter
STATUS_CODE_NO_PASS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8085/uploads/$HASH_ID)
if [ "$STATUS_CODE_NO_PASS" -eq 412 ] || [ "$STATUS_CODE_NO_PASS" -eq 401 ]; then
    echo -e "${GREEN}✔ Test 3.1 passed: Download without password blocked (HTTP status: $STATUS_CODE_NO_PASS).${NC}"
else
    echo -e "${RED}✘ Test 3.1 failed: Download without password returned status $STATUS_CODE_NO_PASS!${NC}"
    exit 1
fi

# Download with wrong password query parameter
STATUS_CODE_WRONG_PASS=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:8085/uploads/$HASH_ID?password=wrong")
if [ "$STATUS_CODE_WRONG_PASS" -eq 401 ]; then
    echo -e "${GREEN}✔ Test 3.2 passed: Download with wrong password blocked (HTTP status: $STATUS_CODE_WRONG_PASS).${NC}"
else
    echo -e "${RED}✘ Test 3.2 failed: Download with wrong password returned status $STATUS_CODE_WRONG_PASS!${NC}"
    exit 1
fi

# Download with correct password query parameter
echo -e "${BLUE}==> Attempting download with correct password...${NC}"
curl -s -o "$TEST_FILES_DIR/downloaded.bin" "http://localhost:8085/uploads/$HASH_ID?password=mysecret123"

# Check file contents
if diff "$TEST_FILE" "$TEST_FILES_DIR/downloaded.bin" >/dev/null; then
    echo -e "${GREEN}✔ Test 3.3 passed: Downloaded file matches original content!${NC}"
else
    echo -e "${RED}✘ Test 3.3 failed: Downloaded file content does not match original!${NC}"
    exit 1
fi

echo -e "\n${GREEN}🎉 All integration tests passed successfully! 🎉${NC}"
rm -f "$TEST_FILES_DIR/downloaded.bin"
