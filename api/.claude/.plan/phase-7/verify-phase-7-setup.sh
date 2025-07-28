#!/bin/bash
# Verification script for Phase 7 planning setup

echo "🔍 Verifying Phase 7 Planning Setup..."
echo "====================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall status
ALL_GOOD=true

# Function to check file exists
check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} $2"
        return 0
    else
        echo -e "${RED}✗${NC} $2"
        ALL_GOOD=false
        return 1
    fi
}

# Function to check file contains text
check_contains() {
    if grep -q "$2" "$1" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} $3"
        return 0
    else
        echo -e "${RED}✗${NC} $3"
        ALL_GOOD=false
        return 1
    fi
}

echo -e "\n📋 Checking Phase 7 Plan Files..."
check_file "WORK_PLAN.md" "WORK_PLAN.md exists"
check_file "REVIEW_PLAN.md" "REVIEW_PLAN.md exists"

echo -e "\n📚 Checking Junior Developer Resources..."
check_file "../../junior-dev-helper/docker-best-practices.md" "Docker Best Practices guide exists"
check_file "../../junior-dev-helper/kubernetes-deployment-guide.md" "Kubernetes Deployment guide exists"
check_file "../../junior-dev-helper/container-security-guide.md" "Container Security guide exists"
check_file "../../junior-dev-helper/secret-management-tutorial.md" "Secret Management tutorial exists"
check_file "../../junior-dev-helper/container-debugging-guide.md" "Container Debugging guide exists"

echo -e "\n🔗 Checking WORK_PLAN.md References..."
check_contains "WORK_PLAN.md" "docker-best-practices.md" "References Docker best practices"
check_contains "WORK_PLAN.md" "kubernetes-deployment-guide.md" "References Kubernetes guide"
check_contains "WORK_PLAN.md" "container-security-guide.md" "References security guide"
check_contains "WORK_PLAN.md" "secret-management-tutorial.md" "References secret management"
check_contains "WORK_PLAN.md" "container-debugging-guide.md" "References debugging guide"

echo -e "\n✅ Checking WORK_PLAN.md Structure..."
check_contains "WORK_PLAN.md" "CHECKPOINT 1: Docker Foundation" "Contains Checkpoint 1"
check_contains "WORK_PLAN.md" "CHECKPOINT 2: Image Optimization" "Contains Checkpoint 2"
check_contains "WORK_PLAN.md" "CHECKPOINT 3: Kubernetes Manifests" "Contains Checkpoint 3"
check_contains "WORK_PLAN.md" "CHECKPOINT 4: Secret Management" "Contains Checkpoint 4"
check_contains "WORK_PLAN.md" "CHECKPOINT 5: Production Readiness" "Contains Checkpoint 5"

echo -e "\n📖 Checking REVIEW_PLAN.md Structure..."
check_contains "REVIEW_PLAN.md" "Docker Foundation Review" "Contains Docker review section"
check_contains "REVIEW_PLAN.md" "Image Optimization Review" "Contains optimization review"
check_contains "REVIEW_PLAN.md" "Kubernetes Manifests Review" "Contains K8s review"
check_contains "REVIEW_PLAN.md" "Secret Management Review" "Contains secret review"
check_contains "REVIEW_PLAN.md" "Production Readiness Review" "Contains production review"

echo -e "\n🎯 Checking Phase 7 Objectives Alignment..."
# Check for key Phase 7 requirements
check_contains "WORK_PLAN.md" "Multi-stage Dockerfile" "Includes multi-stage builds"
check_contains "WORK_PLAN.md" "< 50MB" "Specifies image size requirement"
check_contains "WORK_PLAN.md" "Security scanning" "Includes security scanning"
check_contains "WORK_PLAN.md" "HPA" "Includes Horizontal Pod Autoscaler"
check_contains "WORK_PLAN.md" "Health checks" "Includes health check implementation"

echo -e "\n🔍 Checking TDD Methodology..."
check_contains "WORK_PLAN.md" "Write tests FIRST" "Emphasizes TDD approach"
check_contains "WORK_PLAN.md" "container_tests.rs" "Includes container tests"

echo -e "\n📝 Summary"
echo "========="
if [ "$ALL_GOOD" = true ]; then
    echo -e "${GREEN}✅ All Phase 7 planning files are properly set up!${NC}"
    echo -e "\nPhase 7 is ready for implementation with:"
    echo "- Comprehensive work plan with 5 checkpoints"
    echo "- Detailed review guidelines"
    echo "- 5 junior developer helper guides"
    echo "- Focus on <50MB images with zero vulnerabilities"
    echo "- Complete Kubernetes deployment configuration"
    exit 0
else
    echo -e "${RED}❌ Some Phase 7 files are missing or incomplete${NC}"
    echo -e "\nPlease ensure all files are created and properly referenced."
    exit 1
fi