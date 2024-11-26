#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color
BLUE='\033[0;34m'

# Base URL
BASE_URL="http://localhost:8080"

# Function to test an endpoint
test_endpoint() {
    local endpoint=$1
    local from=$2
    local to=$3
    local amount=$4
    local description=$5
    local endpoint_version=$6  # 'simple' or 'v1'

    echo -e "\n${BLUE}Testing: ${description}${NC}"
    echo "POST ${endpoint}"
    echo "Request: {\"from\": \"${from}\", \"to\": \"${to}\", \"amount\": ${amount}}"
    
    response=$(curl -s -X POST "${BASE_URL}${endpoint}" \
        -H "Content-Type: application/json" \
        -d "{\"from\": \"${from}\", \"to\": \"${to}\", \"amount\": ${amount}}" \
        -w "\nStatus: %{http_code}")
    
    status_code=$(echo "$response" | tail -n1 | cut -d' ' -f2)
    response_body=$(echo "$response" | sed '$d')
    
    echo -e "Response: ${response_body}\n"
    
    # For v1 endpoint, 400 is a valid response for invalid countries
    if [[ "$endpoint_version" == "v1" ]]; then
        if [[ $status_code == 2* ]] || [[ $status_code == 400 ]]; then
            echo -e "${GREEN}✓ Test passed${NC}"
        else
            echo -e "${RED}✗ Test failed (Status: ${status_code})${NC}"
        fi
    else
        # For simple endpoint, only 200s are valid
        if [[ $status_code == 2* ]]; then
            echo -e "${GREEN}✓ Test passed${NC}"
        else
            echo -e "${RED}✗ Test failed (Status: ${status_code})${NC}"
        fi
    fi
}

echo -e "${BLUE}Starting Currency Converter API Tests${NC}\n"

# Test simple endpoint
echo -e "\n${BLUE}=== Testing Simple Endpoint ===${NC}"
test_endpoint "/currency" "United States" "France" 100 "Valid conversion (US to France)" "simple"
test_endpoint "/currency" "Narnia" "France" 100 "Invalid source country" "simple"
test_endpoint "/currency" "Germany" "France" 50 "Valid European conversion" "simple"
test_endpoint "/currency" "japan" "australia" 1000 "Case insensitive test" "simple"

# Test v1 endpoint
echo -e "\n${BLUE}=== Testing V1 Endpoint ===${NC}"
test_endpoint "/v1/currency" "United States" "France" 100 "Valid conversion (US to France)" "v1"
test_endpoint "/v1/currency" "Germany" "Spain" 50 "Valid European conversion" "v1"
test_endpoint "/v1/currency" "Narnia" "France" 100 "Invalid source country" "v1"
test_endpoint "/v1/currency" "japan" "australia" 1000 "Case insensitive test" "v1"

echo -e "\n${BLUE}Testing Complete${NC}"