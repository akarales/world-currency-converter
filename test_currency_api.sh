#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color
BLUE='\033[0;34m'

# Base URL - make configurable
BASE_URL="${API_BASE_URL:-http://localhost:8080}"

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Function to test an endpoint
test_endpoint() {
    local endpoint=$1
    local from=$2
    local to=$3
    local amount=$4
    local description=$5
    local endpoint_version=$6  # 'simple' or 'v1'
    local preferred_currency=$7  # optional

    echo -e "\n${BLUE}Testing: ${description}${NC}"
    echo "POST ${endpoint}"

    local request_body
    if [ -n "$preferred_currency" ]; then
        request_body="{\"from\": \"${from}\", \"to\": \"${to}\", \"amount\": ${amount}, \"preferred_currency\": \"${preferred_currency}\"}"
    else
        request_body="{\"from\": \"${from}\", \"to\": \"${to}\", \"amount\": ${amount}, \"preferred_currency\": null}"
    fi

    echo "Request: ${request_body}"
    
    response=$(curl -s -X POST "${BASE_URL}${endpoint}" \
        -H "Content-Type: application/json" \
        -d "${request_body}" \
        -w "\nStatus: %{http_code}")
    
    status_code=$(echo "$response" | tail -n1 | cut -d' ' -f2)
    response_body=$(echo "$response" | sed '$d')
    
    echo -e "Response: ${response_body}\n"

    # Validate response based on endpoint version
    if [[ "$endpoint_version" == "v1" ]]; then
        # V1 endpoint validation
        if [[ $status_code == 2* ]]; then
            if echo "$response_body" | jq -e '.request_id and .timestamp and .data and .meta' > /dev/null; then
                # For multi-currency tests, verify additional fields if needed
                if [[ "$description" == *"multi-currency"* ]]; then
                    if echo "$response_body" | jq -e '.meta.multiple_currencies_available and .data.available_currencies' > /dev/null; then
                        echo -e "${GREEN}✓ Test passed (multi-currency response verified)${NC}"
                        return 0
                    else
                        echo -e "${RED}✗ Test failed (Missing multi-currency fields)${NC}"
                        return 1
                    fi
                else
                    echo -e "${GREEN}✓ Test passed${NC}"
                    return 0
                fi
            else
                echo -e "${RED}✗ Test failed (Invalid v1 response format)${NC}"
                return 1
            fi
        elif [[ $status_code == 404 && "$description" == *"Invalid source country"* ]]; then
            if echo "$response_body" | jq -e '.error and .request_id and .timestamp' > /dev/null &&
               echo "$response_body" | jq -e '.error | contains("Country not found")' > /dev/null; then
                echo -e "${GREEN}✓ Test passed (Expected 404 for invalid country)${NC}"
                return 0
            else
                echo -e "${RED}✗ Test failed (Invalid error response format)${NC}"
                return 1
            fi
        elif [[ $status_code == 400 ]]; then
            if echo "$response_body" | jq -e '.error and .request_id and .timestamp' > /dev/null; then
                echo -e "${GREEN}✓ Test passed (Expected error response)${NC}"
                return 0
            else
                echo -e "${RED}✗ Test failed (Invalid error response format)${NC}"
                return 1
            fi
        else
            echo -e "${RED}✗ Test failed (Status: ${status_code})${NC}"
            return 1
        fi
    else
        # Simple endpoint validation
        if [[ $status_code == 2* ]]; then
            if echo "$response_body" | jq -e '.from and .to and .amount' > /dev/null; then
                echo -e "${GREEN}✓ Test passed${NC}"
                return 0
            else
                echo -e "${RED}✗ Test failed (Invalid response format)${NC}"
                return 1
            fi
        elif [[ $status_code == 404 && "$description" == *"Invalid source country"* ]]; then
            if echo "$response_body" | jq -e '.error and .request_id and .timestamp' > /dev/null &&
               echo "$response_body" | jq -e '.error | contains("Country not found")' > /dev/null; then
                echo -e "${GREEN}✓ Test passed (Expected 404 for invalid country)${NC}"
                return 0
            else
                echo -e "${RED}✗ Test failed (Invalid error response format)${NC}"
                return 1
            fi
        elif [[ $status_code == 400 ]]; then
            if echo "$response_body" | jq -e '.error and .request_id and .timestamp' > /dev/null; then
                echo -e "${GREEN}✓ Test passed (Expected validation error)${NC}"
                return 0
            else
                echo -e "${RED}✗ Test failed (Invalid error response format)${NC}"
                return 1
            fi
        else
            echo -e "${RED}✗ Test failed (Status: ${status_code})${NC}"
            return 1
        fi
    fi
}

# Check if server is running
echo -e "${BLUE}Checking if server is running...${NC}"
if ! curl -s "http://localhost:8080/health" > /dev/null; then
    echo -e "${RED}Server is not running. Please start the server first with:${NC}"
    echo "cargo run"
    exit 1
fi

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo -e "${RED}jq is required but not installed. Please install jq first:${NC}"
    echo "sudo apt-get install jq  # For Ubuntu/Debian"
    echo "brew install jq         # For macOS"
    exit 1
fi

echo -e "${BLUE}Starting Currency Converter API Tests${NC}\n"

# Initialize counters
total_tests=0
passed_tests=0

# Test simple endpoint
echo -e "\n${BLUE}=== Testing Simple Endpoint ===${NC}"

test_cases=(
    "test_valid_us_france|United States|France|100|Valid conversion (US to France)|simple"
    "test_invalid_country|Narnia|France|100|Invalid source country|simple"
    "test_european|Germany|France|50|Valid European conversion|simple"
    "test_case_sensitive|japan|australia|1000|Case insensitive test|simple"
    "test_zero_amount|United States|France|0|Zero amount validation|simple"
    "test_same_country|France|France|100|Same country conversion|simple"
)

# Run simple endpoint tests
for test_case in "${test_cases[@]}"; do
    IFS='|' read -r test_name from to amount description version <<< "$test_case"
    ((total_tests++))
    if test_endpoint "/currency" "$from" "$to" "$amount" "$description" "$version"; then
        ((passed_tests++))
    fi
done

# Test v1 endpoint
echo -e "\n${BLUE}=== Testing V1 Endpoint ===${NC}"

v1_test_cases=(
    "test_v1_valid_us_france|United States|France|100|Valid conversion (US to France)|v1"
    "test_v1_european|Germany|Spain|50|Valid European conversion|v1"
    "test_v1_invalid_country|Narnia|France|100|Invalid source country|v1"
    "test_v1_case_sensitive|japan|australia|1000|Case insensitive test|v1"
    "test_v1_zero_amount|United States|France|0|Zero amount validation|v1"
    "test_v1_same_country|France|France|100|Same country conversion|v1"
)

# Run v1 endpoint tests
for test_case in "${v1_test_cases[@]}"; do
    IFS='|' read -r test_name from to amount description version <<< "$test_case"
    ((total_tests++))
    if test_endpoint "/v1/currency" "$from" "$to" "$amount" "$description" "$version"; then
        ((passed_tests++))
    fi
done

# Test multi-currency support
echo -e "\n${BLUE}=== Testing Multi-Currency Support ===${NC}"

multi_currency_test_cases=(
    "test_mc_panama_default|Panama|France|100|Panama multi-currency test (default)|v1"
    "test_mc_panama_usd|Panama|France|100|Panama with USD preferred|v1|USD"
    "test_mc_panama_pab|Panama|France|100|Panama with PAB preferred|v1|PAB"
    "test_mc_zimbabwe|Zimbabwe|United States|50|Zimbabwe multi-currency test|v1"
    "test_mc_invalid_currency|Panama|France|100|Invalid preferred currency test|v1|XYZ"
    "test_mc_swiss|Switzerland|France|100|Swiss multi-currency test|v1"
    "test_mc_hongkong|Hong Kong|United States|100|Hong Kong multi-currency test|v1"
)

# Run multi-currency tests
for test_case in "${multi_currency_test_cases[@]}"; do
    IFS='|' read -r test_name from to amount description version preferred_currency <<< "$test_case"
    ((total_tests++))
    if test_endpoint "/v1/currency" "$from" "$to" "$amount" "$description" "$version" "$preferred_currency"; then
        ((passed_tests++))
    fi
done

# Print summary
echo -e "\n${BLUE}=== Test Summary ===${NC}"
echo -e "Total tests: ${total_tests}"
echo -e "Passed tests: ${GREEN}${passed_tests}${NC}"
echo -e "Failed tests: ${RED}$((total_tests - passed_tests))${NC}"

# Set exit code based on test results
if [ $passed_tests -eq $total_tests ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed.${NC}"
    exit 1
fi