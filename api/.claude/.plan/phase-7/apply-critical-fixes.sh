#!/bin/bash
# Script to apply critical fixes to Phase 7 plans

set -euo pipefail

echo "🔧 Applying critical fixes to Phase 7 plans..."
echo "============================================"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Fix health check timeouts in WORK_PLAN.md
echo -e "\n${YELLOW}Fixing health check timeouts in WORK_PLAN.md...${NC}"

# Fix Docker HEALTHCHECK timeout
if grep -q "timeout=3s" WORK_PLAN.md; then
    sed -i.bak 's/timeout=3s/timeout=5s/g' WORK_PLAN.md
    echo -e "${GREEN}✓${NC} Updated Docker HEALTHCHECK timeout from 3s to 5s"
fi

# Add comment about SPEC requirement
if ! grep -q "Timeout must be 5s per SPEC.md requirement" WORK_PLAN.md; then
    sed -i.bak '/HEALTHCHECK.*timeout=5s/a\
    # Timeout must be 5s per SPEC.md requirement' WORK_PLAN.md
    echo -e "${GREEN}✓${NC} Added SPEC requirement comment"
fi

# Fix health check timeouts in REVIEW_PLAN.md
echo -e "\n${YELLOW}Fixing health check timeouts in REVIEW_PLAN.md...${NC}"

if grep -q "timeout=3s" REVIEW_PLAN.md; then
    sed -i.bak 's/timeout=3s/timeout=5s/g' REVIEW_PLAN.md
    echo -e "${GREEN}✓${NC} Updated health check timeout from 3s to 5s in REVIEW_PLAN.md"
fi

# Create backup directory
mkdir -p .backups
mv *.bak .backups/ 2>/dev/null || true

echo -e "\n${GREEN}✅ Critical fixes applied successfully!${NC}"
echo -e "Backups saved in .backups/ directory"

# Verify the changes
echo -e "\n${YELLOW}Verifying changes...${NC}"
echo "Health check timeouts in WORK_PLAN.md:"
grep -n "timeout=" WORK_PLAN.md | grep -i health | head -5 || true

echo -e "\nHealth check timeouts in REVIEW_PLAN.md:"
grep -n "timeout=" REVIEW_PLAN.md | grep -i health | head -5 || true

echo -e "\n${GREEN}🎉 Done! Critical fixes have been applied.${NC}"
echo "Next steps:"
echo "1. Review the changes"
echo "2. Add the troubleshooting section manually (see GAP_REMEDIATION_PLAN.md)"
echo "3. Run verify-phase-7-setup.sh to confirm everything still validates"