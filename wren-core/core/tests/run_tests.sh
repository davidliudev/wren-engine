#!/bin/bash

# Script to run all wren-core tests
# Usage: ./run_tests.sh [test_filter]
# Example: ./run_tests.sh                  # Run all tests
# Example: ./run_tests.sh extract_schema    # Run only schema extraction tests
# Example: ./run_tests.sh validate          # Run only validation tests

set -e

echo "🧪 Running Wren Core Tests"
echo "========================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Change to project root
cd "$PROJECT_ROOT"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found. Please run this script from the wren-core directory.${NC}"
    exit 1
fi

# Optional test filter from command line argument
TEST_FILTER=""
if [ $# -gt 0 ]; then
    TEST_FILTER="$1"
    echo -e "${YELLOW}Running tests matching: $TEST_FILTER${NC}"
fi

# Build the library first
echo -e "\n${YELLOW}Building wren-core library...${NC}"
cargo build --release

# Run the tests
echo -e "\n${YELLOW}Running tests...${NC}"

if [ -z "$TEST_FILTER" ]; then
    # Run all tests with verbose output
    cargo test --release -- --nocapture --test-threads=1
else
    # Run filtered tests
    cargo test --release "$TEST_FILTER" -- --nocapture --test-threads=1
fi

# Check the exit code
if [ $? -eq 0 ]; then
    echo -e "\n${GREEN}✅ All tests passed!${NC}"
else
    echo -e "\n${RED}❌ Some tests failed.${NC}"
    exit 1
fi

# Optional: Generate test coverage report (requires cargo-tarpaulin)
if command -v cargo-tarpaulin &> /dev/null; then
    echo -e "\n${YELLOW}Generating test coverage report...${NC}"
    cargo tarpaulin --out Html --output-dir target/coverage
    echo -e "${GREEN}Coverage report generated at: target/coverage/tarpaulin-report.html${NC}"
fi

echo -e "\n${GREEN}Test run completed successfully!${NC}"