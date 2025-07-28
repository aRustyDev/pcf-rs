#!/bin/bash
# Phase 6 Performance Verification Script Template
#
# This script verifies all Phase 6 performance optimizations are correctly implemented.
# Customize the paths and endpoints for your specific setup.

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
API_ENDPOINT="${API_ENDPOINT:-http://localhost:8080}"
GRAPHQL_ENDPOINT="${API_ENDPOINT}/graphql"
METRICS_ENDPOINT="${API_ENDPOINT}/metrics"
ADMIN_TOKEN="${ADMIN_TOKEN:-test-admin-token}"

# Counters
PASSED=0
FAILED=0

echo "🚀 Phase 6 Performance Verification"
echo "=================================="
echo "API Endpoint: $API_ENDPOINT"
echo ""

# Helper function for tests
run_test() {
    local test_name=$1
    local test_command=$2
    local expected=$3
    
    echo -n "Testing $test_name... "
    
    if eval "$test_command"; then
        echo -e "${GREEN}PASSED${NC}"
        ((PASSED++))
    else
        echo -e "${RED}FAILED${NC}"
        echo "  Expected: $expected"
        ((FAILED++))
    fi
}

# Helper to check if service is running
check_service() {
    if ! curl -s -f "$API_ENDPOINT/health" > /dev/null; then
        echo -e "${RED}ERROR: API service is not running at $API_ENDPOINT${NC}"
        exit 1
    fi
}

# Pre-flight check
check_service

echo "📋 Checkpoint 1: DataLoader Implementation"
echo "----------------------------------------"

# Test 1.1: Check N+1 query prevention
echo "Testing N+1 query prevention..."
QUERY='{ users(first: 10) { id name notes { id title author { name } } } }'
START_TIME=$(date +%s.%N)

# Make request and capture metrics
METRICS_BEFORE=$(curl -s "$METRICS_ENDPOINT" | grep -E "db_queries_total|dataloader_batch_count" | awk '{print $2}')
curl -s -X POST "$GRAPHQL_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"query\": \"$QUERY\"}" > /dev/null
METRICS_AFTER=$(curl -s "$METRICS_ENDPOINT" | grep -E "db_queries_total|dataloader_batch_count" | awk '{print $2}')

END_TIME=$(date +%s.%N)
DURATION=$(echo "$END_TIME - $START_TIME" | bc)

# Check if queries were batched (should be < 5 queries for this request)
DB_QUERIES=$(echo "$METRICS_AFTER - $METRICS_BEFORE" | bc)
run_test "N+1 prevention (queries < 5)" "[ $DB_QUERIES -lt 5 ]" "Less than 5 DB queries"

# Test 1.2: Check DataLoader metrics exist
run_test "DataLoader metrics exported" \
    "curl -s '$METRICS_ENDPOINT' | grep -q 'dataloader_batch_size'" \
    "DataLoader metrics present"

# Test 1.3: Verify batch efficiency
BATCH_EFFICIENCY=$(curl -s "$METRICS_ENDPOINT" | grep dataloader_batch_efficiency | awk '{print $2}')
run_test "Batch efficiency > 5" "[ ${BATCH_EFFICIENCY:-0} -gt 5 ]" "Efficiency > 5x"

echo ""
echo "📋 Checkpoint 2: Response Caching"
echo "--------------------------------"

# Test 2.1: Cache warming
echo "Testing cache effectiveness..."
TEST_QUERY='{ user(id: "1") { id name notes { title } } }'

# First request (cache miss)
TIME1=$(curl -s -X POST "$GRAPHQL_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"query\": \"$TEST_QUERY\"}" \
    -w "%{time_total}" -o /dev/null)

# Second request (should be cache hit)
TIME2=$(curl -s -X POST "$GRAPHQL_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"query\": \"$TEST_QUERY\"}" \
    -w "%{time_total}" -o /dev/null)

# Cache should make second request at least 10x faster
SPEEDUP=$(echo "scale=2; $TIME1 / $TIME2" | bc)
run_test "Cache speedup > 10x" "[ $(echo "$SPEEDUP > 10" | bc) -eq 1 ]" "Second request 10x faster"

# Test 2.2: Cache isolation
USER1_QUERY='{ currentUser { privateNotes { id } } }'
USER1_RESULT=$(curl -s -X POST "$GRAPHQL_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer user1-token" \
    -d "{\"query\": \"$USER1_QUERY\"}")

USER2_RESULT=$(curl -s -X POST "$GRAPHQL_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer user2-token" \
    -d "{\"query\": \"$USER1_QUERY\"}")

run_test "Cache isolation between users" \
    "[ '$USER1_RESULT' != '$USER2_RESULT' ]" \
    "Different results for different users"

# Test 2.3: Cache metrics
CACHE_HIT_RATE=$(curl -s "$METRICS_ENDPOINT" | grep response_cache_hit_rate | awk '{print $2}')
run_test "Cache hit rate > 0" "[ ${CACHE_HIT_RATE:-0} -gt 0 ]" "Cache being used"

echo ""
echo "📋 Checkpoint 3: Timeout Implementation"
echo "--------------------------------------"

# Test 3.1: Timeout cascade
echo "Testing timeout cascade..."
SLOW_QUERY='{ slowQuery(delay: 35) }'
START=$(date +%s)
HTTP_CODE=$(curl -s -X POST "$GRAPHQL_ENDPOINT" \
    -H "Content-Type: application/json" \
    -d "{\"query\": \"$SLOW_QUERY\"}" \
    -m 40 \
    -w "%{http_code}" -o /dev/null)
END=$(date +%s)
DURATION=$((END - START))

run_test "Request times out < 35s" "[ $DURATION -lt 35 ]" "Timeout before 35s"
run_test "Returns timeout error" "[ $HTTP_CODE -eq 408 ]" "HTTP 408 status"

# Test 3.2: Timeout metrics
TIMEOUT_COUNT=$(curl -s "$METRICS_ENDPOINT" | grep request_timeouts_total | awk '{print $2}')
run_test "Timeout metrics recorded" "[ ${TIMEOUT_COUNT:-0} -gt 0 ]" "Timeouts tracked"

echo ""
echo "📋 Checkpoint 4: Load Test Performance"
echo "-------------------------------------"

# Test 4.1: Basic load test
echo "Running mini load test (100 RPS for 10s)..."
if command -v vegeta &> /dev/null; then
    echo '{"query":"{ users(first: 5) { id name } }"}' | \
    vegeta attack -format=json -rate=100 -duration=10s -timeout=5s \
        -header="Content-Type: application/json" \
        -targets=- -output=results.bin \
        "POST $GRAPHQL_ENDPOINT" 2>/dev/null
    
    P99=$(vegeta report -type=json results.bin 2>/dev/null | jq '.latencies.p99 / 1000000')
    SUCCESS_RATE=$(vegeta report -type=json results.bin 2>/dev/null | jq '.success')
    
    run_test "P99 latency < 200ms" "[ $(echo "$P99 < 200" | bc) -eq 1 ]" "P99 < 200ms"
    run_test "Success rate > 99%" "[ $(echo "$SUCCESS_RATE > 0.99" | bc) -eq 1 ]" ">99% success"
    
    rm -f results.bin
else
    echo -e "${YELLOW}WARNING: vegeta not installed, skipping load test${NC}"
fi

# Test 4.2: Connection pool health
POOL_ACTIVE=$(curl -s "$METRICS_ENDPOINT" | grep db_pool_connections_active | awk '{print $2}')
POOL_IDLE=$(curl -s "$METRICS_ENDPOINT" | grep db_pool_connections_idle | awk '{print $2}')

run_test "Connection pool has active connections" "[ ${POOL_ACTIVE:-0} -gt 0 ]" "Active connections"
run_test "Connection pool has idle connections" "[ ${POOL_IDLE:-0} -gt 0 ]" "Idle connections"

# Test 4.3: Metric cardinality check
echo ""
echo "Checking metric cardinality..."
for metric in $(curl -s "$METRICS_ENDPOINT" | grep -E "^graphql_" | cut -d'{' -f1 | sort | uniq); do
    count=$(curl -s "$METRICS_ENDPOINT" | grep "^$metric" | wc -l)
    if [ $count -gt 1000 ]; then
        echo -e "${RED}WARNING: $metric has $count labels (exceeds 1000 limit)${NC}"
        ((FAILED++))
    else
        echo -e "${GREEN}✓ $metric: $count labels${NC}"
    fi
done

echo ""
echo "📊 Final Report"
echo "==============="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ Phase 6 Verification PASSED!${NC}"
    echo "All performance optimizations are working correctly."
    exit 0
else
    echo ""
    echo -e "${RED}❌ Phase 6 Verification FAILED${NC}"
    echo "Please fix the failing tests before proceeding."
    exit 1
fi

# Cleanup helper script
cat > verify-phase-6-cleanup.sh << 'EOF'
#!/bin/bash
# Cleanup script for Phase 6 verification

# Reset metrics
curl -X POST http://localhost:8080/admin/metrics/reset

# Clear caches
curl -X POST http://localhost:8080/admin/cache/clear

# Reset connection pools
curl -X POST http://localhost:8080/admin/pools/reset

echo "Phase 6 test environment reset"
EOF

chmod +x verify-phase-6-cleanup.sh
echo ""
echo "💡 Run ./verify-phase-6-cleanup.sh to reset test environment"