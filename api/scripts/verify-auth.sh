#!/bin/bash

# PCF API Authorization Verification Script
#
# This script provides a convenient wrapper around the Python verification script
# and includes additional checks using curl and other command-line tools.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
HOST="localhost:8080"
PROTOCOL="http"
TIMEOUT=30

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            HOST="$2"
            shift 2
            ;;
        --ssl)
            PROTOCOL="https"
            shift
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --host HOST     API host and port (default: localhost:8080)"
            echo "  --ssl           Use HTTPS instead of HTTP"
            echo "  --timeout SEC   Request timeout in seconds (default: 30)"
            echo "  --help, -h      Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

BASE_URL="${PROTOCOL}://${HOST}"

echo -e "${BLUE}🔐 PCF API Authorization Verification${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""
echo -e "Testing API at: ${YELLOW}${BASE_URL}${NC}"
echo ""

# Check if API is reachable
echo -e "${BLUE}1. Basic Connectivity Check${NC}"
if curl -s --connect-timeout "$TIMEOUT" "${BASE_URL}/health" > /dev/null 2>&1; then
    echo -e "   ${GREEN}✅ API is reachable${NC}"
else
    echo -e "   ${RED}❌ API is not reachable${NC}"
    echo -e "   ${YELLOW}💡 Make sure the API server is running on ${HOST}${NC}"
    exit 1
fi

# Check GraphQL endpoint
echo -e "${BLUE}2. GraphQL Endpoint Check${NC}"
GRAPHQL_RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"query": "{ __schema { queryType { name } } }"}' \
    --connect-timeout "$TIMEOUT" \
    "${BASE_URL}/graphql" || echo "ERROR")

if [[ "$GRAPHQL_RESPONSE" == "ERROR" ]]; then
    echo -e "   ${RED}❌ GraphQL endpoint is not accessible${NC}"
    exit 1
elif echo "$GRAPHQL_RESPONSE" | grep -q "queryType"; then
    echo -e "   ${GREEN}✅ GraphQL endpoint is responding${NC}"
else
    echo -e "   ${YELLOW}⚠️  GraphQL endpoint responded but format is unexpected${NC}"
fi

# Check if demo mode is enabled (look for warning indicators)
echo -e "${BLUE}3. Demo Mode Detection${NC}"
# This is a basic check - in a real implementation, you might check logs or metrics endpoints
if curl -s "${BASE_URL}/graphql" -d '{"query": "{ health { status } }"}' | grep -q "demo" 2>/dev/null; then
    echo -e "   ${YELLOW}⚠️  Demo mode may be enabled (check server logs)${NC}"
    echo -e "   ${YELLOW}💡 Ensure demo mode is disabled in production!${NC}"
else
    echo -e "   ${GREEN}✅ No obvious demo mode indicators found${NC}"
fi

# Test authorization on protected endpoints
echo -e "${BLUE}4. Authorization Requirements Check${NC}"
AUTH_TEST_RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"query": "{ notes(first: 1) { edges { node { id } } } }"}' \
    --connect-timeout "$TIMEOUT" \
    "${BASE_URL}/graphql")

if echo "$AUTH_TEST_RESPONSE" | grep -qi "auth\|permission\|unauthorized"; then
    echo -e "   ${GREEN}✅ Protected endpoints require authorization${NC}"
else
    echo -e "   ${RED}❌ Protected endpoints may not require authorization${NC}"
    echo -e "   ${RED}🚨 SECURITY ISSUE: Investigate immediately!${NC}"
fi

# Run the comprehensive Python verification script
echo -e "${BLUE}5. Running Comprehensive Verification${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if command -v python3 &> /dev/null; then
    if [[ "$PROTOCOL" == "https" ]]; then
        python3 "${SCRIPT_DIR}/verify_authorization.py" --host "$HOST" --ssl --timeout "$TIMEOUT"
    else
        python3 "${SCRIPT_DIR}/verify_authorization.py" --host "$HOST" --timeout "$TIMEOUT"
    fi
else
    echo -e "   ${YELLOW}⚠️  Python3 not found, skipping detailed verification${NC}"
    echo -e "   ${YELLOW}💡 Install Python3 to run comprehensive tests${NC}"
fi

echo ""
echo -e "${BLUE}📋 Security Checklist${NC}"
echo -e "${BLUE}====================${NC}"
echo ""
echo -e "Manual checks to perform:"
echo -e "  ${YELLOW}□${NC} Verify HTTPS is used in production"
echo -e "  ${YELLOW}□${NC} Check that demo mode is disabled in production"
echo -e "  ${YELLOW}□${NC} Verify rate limiting is configured appropriately"
echo -e "  ${YELLOW}□${NC} Review authorization audit logs"
echo -e "  ${YELLOW}□${NC} Test with actual user credentials"
echo -e "  ${YELLOW}□${NC} Verify SpiceDB connection is secure"
echo ""
echo -e "${GREEN}✅ Authorization verification completed!${NC}"