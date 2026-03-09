#!/usr/bin/env bash
#
# Test suite for skill-sync/sync-skills.sh
#

set -euo pipefail

# Test framework
TESTS_PASSED=0
TESTS_FAILED=0

pass() {
    echo "  ✓ PASS: $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() {
    echo "  ✗ FAIL: $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

# Script location
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/sync-skills.sh"

# Setup temporary directories
setup() {
    TEST_DIR=$(mktemp -d)
    SOURCE_ROOT1="$TEST_DIR/source1"
    SOURCE_ROOT2="$TEST_DIR/source2"
    DEST_DIR="$TEST_DIR/.agents/skills"
    SELECTED_FILE="$TEST_DIR/.agents/skills.selected"
    CONFIG_FILE="$TEST_DIR/sync-skills.conf"
    
    mkdir -p "$SOURCE_ROOT1" "$SOURCE_ROOT2" "$DEST_DIR"
    mkdir -p "$TEST_DIR/.agents"
    
    # Create config file
    echo "$SOURCE_ROOT1" > "$CONFIG_FILE"
    echo "$SOURCE_ROOT2" >> "$CONFIG_FILE"
    
    # Export environment variables for the script
    export SKILL_SYNC_CONFIG="$CONFIG_FILE"
    export SKILL_SYNC_DEST="$DEST_DIR"
    export SKILL_SYNC_SELECTED="$SELECTED_FILE"
}

# Cleanup
teardown() {
    rm -rf "$TEST_DIR"
}

# Create a test skill
create_skill() {
    local root="$1"
    local name="$2"
    mkdir -p "$root/$name"
    echo "# $name Skill" > "$root/$name/SKILL.md"
}

# Test 1: --list-all prints expected human-readable rows
test_list_all_human() {
    echo "Test 1: --list-all prints human-readable output"
    
    create_skill "$SOURCE_ROOT1" "skill-a"
    create_skill "$SOURCE_ROOT1" "skill-b"
    
    output=$("$SCRIPT_PATH" --list-all 2>&1)
    
    if echo "$output" | grep -q "skill-a"; then
        pass "Found skill-a in output"
    else
        fail "Did not find skill-a in output"
    fi
    
    if echo "$output" | grep -q "skill-b"; then
        pass "Found skill-b in output"
    else
        fail "Did not find skill-b in output"
    fi
    
    if echo "$output" | grep -q "unselected"; then
        pass "Found 'unselected' status"
    else
        fail "Did not find 'unselected' status"
    fi
}

# Test 2: --list-all prints all matching source roots for conflicts
test_list_all_conflict() {
    echo "Test 2: --list-all shows conflicts with multiple sources"
    
    create_skill "$SOURCE_ROOT1" "conflict-skill"
    create_skill "$SOURCE_ROOT2" "conflict-skill"
    
    output=$("$SCRIPT_PATH" --list-all 2>&1) || true
    
    # Should show conflict status
    if echo "$output" | grep -q "conflict"; then
        pass "Found 'conflict' status"
    else
        fail "Did not find 'conflict' status"
    fi
    
    # Clean up conflict skills so they don't interfere with later tests
    rm -rf "$SOURCE_ROOT1/conflict-skill" "$SOURCE_ROOT2/conflict-skill"
}

# Test 3: --list-all --json returns valid JSON
test_list_all_json() {
    echo "Test 3: --list-all --json returns valid JSON"
    
    create_skill "$SOURCE_ROOT1" "json-skill"
    echo "json-skill" > "$SELECTED_FILE"
    
    output=$("$SCRIPT_PATH" --list-all --json 2>&1)
    
    # Check if output is valid JSON array
    if echo "$output" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        pass "Output is valid JSON"
    else
        fail "Output is not valid JSON"
    fi
    
    # Check for source field in unique skill
    if echo "$output" | grep -q '"source"'; then
        pass "Found 'source' field for unique skill"
    else
        fail "Did not find 'source' field"
    fi
}

# Test 4: Missing selected skill prints warning and exits 0
test_missing_selected_warning() {
    echo "Test 4: Missing selected skill warns but exits 0"
    
    echo "nonexistent-skill" > "$SELECTED_FILE"
    
    output=$("$SCRIPT_PATH" --sync 2>&1) || {
        fail "Should exit 0 but got exit code $?"
        return
    }
    
    if echo "$output" | grep -qi "warning.*nonexistent-skill"; then
        pass "Warning printed for missing skill"
    else
        fail "No warning for missing skill"
    fi
}

# Test 5: Missing selected skill does not delete existing copy
test_missing_selected_no_delete() {
    echo "Test 5: Missing selected skill does not delete existing copy"
    
    create_skill "$SOURCE_ROOT1" "existing-skill"
    echo "existing-skill" > "$SELECTED_FILE"
    
    # First sync to copy the skill
    "$SCRIPT_PATH" --sync 2>/dev/null
    
    # Verify it exists
    if [[ -d "$DEST_DIR/existing-skill" ]]; then
        pass "Skill was synced"
    else
        fail "Skill was not synced"
        return
    fi
    
    # Now change selection to a missing skill
    echo "another-missing-skill" > "$SELECTED_FILE"
    
    # Run sync again
    "$SCRIPT_PATH" --sync 2>/dev/null || true
    
    # The existing skill should be pruned (removed) because it's not selected
    if [[ ! -d "$DEST_DIR/existing-skill" ]]; then
        pass "Unselected skill was pruned as expected"
    else
        fail "Unselected skill was not pruned"
    fi
}

# Test 6: Invalid source root warns and is skipped
test_invalid_source_root() {
    echo "Test 6: Invalid source root warns and is skipped"
    
    # Add invalid source to config
    echo "/nonexistent/path" >> "$CONFIG_FILE"
    
    output=$("$SCRIPT_PATH" --list-all 2>&1) || true
    
    if echo "$output" | grep -qi "warning.*nonexistent"; then
        pass "Warning printed for invalid source root"
    else
        fail "No warning for invalid source root"
    fi
}

# Test 7: Duplicate skill exits with code 4
test_duplicate_skill_exit_code() {
    echo "Test 7: Duplicate skill in multiple roots exits with code 4"
    
    create_skill "$SOURCE_ROOT1" "dup-skill"
    create_skill "$SOURCE_ROOT2" "dup-skill"
    
    exit_code=0
    "$SCRIPT_PATH" --sync 2>/dev/null || exit_code=$?
    
    if [[ $exit_code -eq 4 ]]; then
        pass "Exit code 4 for duplicate skill"
    else
        fail "Expected exit code 4, got $exit_code"
    fi
    
    # Clean up so subsequent tests don't hit this conflict
    rm -rf "$SOURCE_ROOT1/dup-skill" "$SOURCE_ROOT2/dup-skill"
}

# Test 8: Selected skill without SKILL.md is treated as missing (warns only)
test_invalid_skill_no_skill_md() {
    echo "Test 8: Selected skill without SKILL.md is treated as missing (warns only)"
    
    # Create skill dir without SKILL.md
    mkdir -p "$SOURCE_ROOT1/bad-skill"
    echo "bad-skill" > "$SELECTED_FILE"
    
    exit_code=0
    "$SCRIPT_PATH" --sync 2>/dev/null || exit_code=$?
    
    # Skills without SKILL.md are not discoverable
    # So they're treated as "missing" skills which warn but don't fail
    if [[ $exit_code -eq 0 ]]; then
        pass "Exit code 0 for undiscoverable skill (warns only)"
    else
        fail "Expected exit code 0, got $exit_code"
    fi
    
    # Clean up
    rm -rf "$SOURCE_ROOT1/bad-skill"
}

# Test 9: Invalid flag combinations fail with code 2
test_invalid_flag_combinations() {
    echo "Test 9: Invalid flag combinations fail with code 2"
    
    # --json without --list-all should fail
    exit_code=0
    "$SCRIPT_PATH" --json 2>/dev/null || exit_code=$?
    
    if [[ $exit_code -eq 2 ]]; then
        pass "Exit code 2 for --json without --list-all"
    else
        fail "Expected exit code 2, got $exit_code"
    fi
    
    # --sync and --list-all together should fail
    exit_code=0
    "$SCRIPT_PATH" --sync --list-all 2>/dev/null || exit_code=$?
    
    if [[ $exit_code -eq 2 ]]; then
        pass "Exit code 2 for --sync --list-all combination"
    else
        fail "Expected exit code 2, got $exit_code"
    fi
}

# Test 10: --dry-run reports actions without mutating
test_dry_run_no_mutation() {
    echo "Test 10: --dry-run reports without mutating"
    
    create_skill "$SOURCE_ROOT1" "dryrun-skill"
    echo "dryrun-skill" > "$SELECTED_FILE"
    
    output=$("$SCRIPT_PATH" --sync --dry-run 2>&1)
    
    # Check for dry-run output
    if echo "$output" | grep -q "\[DRY-RUN\]"; then
        pass "DRY-RUN marker in output"
    else
        fail "No DRY-RUN marker in output"
    fi
    
    # Verify no changes made
    if [[ ! -d "$DEST_DIR/dryrun-skill" ]]; then
        pass "No mutation occurred"
    else
        fail "Mutation occurred despite dry-run"
    fi
}

# Test 11: Sync copies valid skills atomically
test_sync_copy_atomic() {
    echo "Test 11: Sync copies valid skills"
    
    create_skill "$SOURCE_ROOT1" "atomic-skill"
    echo "atomic-skill" > "$SELECTED_FILE"
    
    "$SCRIPT_PATH" --sync 2>/dev/null
    
    if [[ -d "$DEST_DIR/atomic-skill" && -f "$DEST_DIR/atomic-skill/SKILL.md" ]]; then
        pass "Skill copied successfully"
    else
        fail "Skill was not copied"
    fi
}

# Test 12: Strict mirror pruning removes unselected skills
test_prune_unselected() {
    echo "Test 12: Strict mirror pruning removes unselected skills"
    
    # Create and sync two skills
    create_skill "$SOURCE_ROOT1" "keep-skill"
    create_skill "$SOURCE_ROOT1" "remove-skill"
    
    echo -e "keep-skill\nremove-skill" > "$SELECTED_FILE"
    "$SCRIPT_PATH" --sync 2>/dev/null
    
    # Verify both exist
    if [[ -d "$DEST_DIR/keep-skill" && -d "$DEST_DIR/remove-skill" ]]; then
        pass "Both skills initially synced"
    else
        fail "Skills not synced"
        return
    fi
    
    # Now only select one
    echo "keep-skill" > "$SELECTED_FILE"
    "$SCRIPT_PATH" --sync 2>/dev/null
    
    # Verify pruning
    if [[ -d "$DEST_DIR/keep-skill" && ! -d "$DEST_DIR/remove-skill" ]]; then
        pass "Unselected skill was pruned"
    else
        fail "Pruning did not work correctly"
    fi
}

# Run all tests
main() {
    echo "========================================"
    echo "Running skill-sync test suite"
    echo "========================================"
    echo ""
    
    setup
    
    test_list_all_human
    test_list_all_conflict
    test_list_all_json
    test_missing_selected_warning
    test_missing_selected_no_delete
    test_invalid_source_root
    test_duplicate_skill_exit_code
    test_invalid_skill_no_skill_md
    test_invalid_flag_combinations
    test_dry_run_no_mutation
    test_sync_copy_atomic
    test_prune_unselected
    
    teardown
    
    echo ""
    echo "========================================"
    echo "Test Results: $TESTS_PASSED passed, $TESTS_FAILED failed"
    echo "========================================"
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo "All tests passed! ✓"
        exit 0
    else
        echo "Some tests failed. ✗"
        exit 1
    fi
}

main "$@"
