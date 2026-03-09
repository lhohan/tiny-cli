#!/usr/bin/env bash
#
# skill-sync/sync-skills.sh - Config-driven local skill sync tool
#
# Syncs selected local skills into .agents/skills based on configured source roots.
#

set -euo pipefail

# Script directory and config paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${SKILL_SYNC_CONFIG:-${SCRIPT_DIR}/sync-skills.conf}"
DEST_DIR="${SKILL_SYNC_DEST:-${SCRIPT_DIR}/../.agents/skills}"
SELECTED_FILE="${SKILL_SYNC_SELECTED:-${SCRIPT_DIR}/../.agents/skills.selected}"

# Flags
MODE=""
JSON_OUTPUT=false
DRY_RUN=false

# Show usage
usage() {
    cat << 'EOF'
Usage: sync-skills.sh [OPTIONS]

Sync selected local skills into .agents/skills based on configured source roots.

Options:
  --sync        Sync mode (default when no mode flag provided)
  --list-all    List all discovered skills across configured roots
  --json        Output as JSON (only valid with --list-all)
  --dry-run     Show planned actions without making changes
  --help        Show this help message

Exit codes:
  0  Success (including warning-only runs)
  2  Usage or validation error
  3  Configuration or environment prerequisite error
  4  Runtime failure (e.g., duplicate skill conflicts)

Config files:
  sync-skills.conf    Source root configuration (one path per line)
  .agents/skills.selected  Selected skills to sync (one name per line)
EOF
}

# Error and exit functions
error() {
    echo "ERROR: $1" >&2
    exit "${2:-2}"
}

warn() {
    echo "WARNING: $1" >&2
}

# Parse arguments
parse_args() {
    local has_mode=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --sync)
                if [[ "$has_mode" == true ]]; then
                    error "Cannot specify multiple mode flags" 2
                fi
                MODE="sync"
                has_mode=true
                ;;
            --list-all)
                if [[ "$has_mode" == true ]]; then
                    error "Cannot specify multiple mode flags" 2
                fi
                MODE="list"
                has_mode=true
                ;;
            --json)
                JSON_OUTPUT=true
                ;;
            --dry-run)
                DRY_RUN=true
                ;;
            --help)
                usage
                exit 0
                ;;
            *)
                error "Unknown option: $1" 2
                ;;
        esac
        shift
    done
    
    # Default to sync mode if no mode specified
    if [[ -z "$MODE" ]]; then
        MODE="sync"
    fi
    
    # Validate flag combinations
    if [[ "$JSON_OUTPUT" == true && "$MODE" != "list" ]]; then
        error "--json is only valid with --list-all" 2
    fi
}

# Read config file and return valid source roots
# Skips comments, blank lines, and invalid entries
read_source_roots() {
    local config_file="$1"
    local -n roots_ref=$2
    
    if [[ ! -f "$config_file" ]]; then
        error "Config file not found: $config_file" 3
    fi
    
    if [[ ! -r "$config_file" ]]; then
        error "Config file not readable: $config_file" 3
    fi
    
    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and blank lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue
        
        # Expand ~ to home directory
        line="${line/#\~/$HOME}"
        
        # Trim leading/trailing whitespace
        line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        
        # Skip if not a directory or not readable
        if [[ ! -d "$line" ]]; then
            warn "Source root is not a directory, skipping: $line"
            continue
        fi
        
        if [[ ! -r "$line" ]]; then
            warn "Source root is not readable, skipping: $line"
            continue
        fi
        
        roots_ref+=("$line")
    done < "$config_file"
    
    if [[ ${#roots_ref[@]} -eq 0 ]]; then
        error "No valid source roots configured in $config_file" 3
    fi
}

# Read selected skills from file
read_selected_skills() {
    local selected_file="$1"
    local -n skills_ref=$2
    
    if [[ ! -f "$selected_file" ]]; then
        # Empty selection is valid
        return
    fi
    
    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and blank lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue
        
        # Trim whitespace
        line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        
        skills_ref+=("$line")
    done < "$selected_file"
}

# Discover all skills across source roots
# Returns array of "name:source_root" pairs
discover_skills() {
    local -n roots_ref=$1
    local -n discovered_ref=$2
    
    for root in "${roots_ref[@]}"; do
        # Look for immediate child directories with SKILL.md
        for skill_dir in "$root"/*/; do
            # Remove trailing slash
            skill_dir="${skill_dir%/}"
            
            # Skip if not a directory or doesn't exist
            [[ ! -d "$skill_dir" ]] && continue
            
            # Get skill name from directory name
            local skill_name
            skill_name="$(basename "$skill_dir")"
            
            # Check if it has SKILL.md
            if [[ -f "$skill_dir/SKILL.md" ]]; then
                discovered_ref+=("$skill_name:$root")
            fi
        done
    done
}

# Check for conflicts (same skill in multiple roots)
# Returns 0 if no conflicts, 1 if conflicts found
# Prints conflicts to stderr
check_conflicts() {
    local -n discovered_ref=$1
    local -n conflicts_ref=$2
    local has_conflict=false
    
    # Build a map of skill name to array of roots
    declare -A skill_roots
    
    for entry in "${discovered_ref[@]}"; do
        local name="${entry%%:*}"
        local root="${entry#*:}"
        
        if [[ -n "${skill_roots[$name]:-}" ]]; then
            skill_roots[$name]="${skill_roots[$name]}|$root"
        else
            skill_roots[$name]="$root"
        fi
    done
    
    # Check for conflicts
    for name in "${!skill_roots[@]}"; do
        local roots_str="${skill_roots[$name]}"
        if [[ "$roots_str" == *"|"* ]]; then
            has_conflict=true
            conflicts_ref+=("$name:$roots_str")
        fi
    done
    
    # Return 1 (failure) if conflicts found, 0 (success) if no conflicts
    if [[ "$has_conflict" == true ]]; then
        return 1
    else
        return 0
    fi
}

# Build canonical dataset for listing
# Returns array of entries with status
build_list_data() {
    local -n discovered_ref=$1
    local -n selected_ref=$2
    local -n data_ref=$3
    
    declare -A skill_status
    declare -A skill_sources
    
    # Build map of discovered skills
    for entry in "${discovered_ref[@]}"; do
        local name="${entry%%:*}"
        local root="${entry#*:}"
        
        if [[ -n "${skill_sources[$name]:-}" ]]; then
            skill_sources[$name]="${skill_sources[$name]}|$root"
        else
            skill_sources[$name]="$root"
        fi
    done
    
    # Build selected set
    declare -A selected_set
    for skill in "${selected_ref[@]}"; do
        selected_set[$skill]=1
    done
    
    # Determine status for each discovered skill
    for name in "${!skill_sources[@]}"; do
        local sources="${skill_sources[$name]}"
        local status
        
        if [[ "$sources" == *"|"* ]]; then
            status="conflict"
        elif [[ -n "${selected_set[$name]:-}" ]]; then
            status="selected"
        else
            status="unselected"
        fi
        
        data_ref+=("$name:$status:$sources")
    done
}

# Output list in human-readable format
output_human_list() {
    local -n data_ref=$1
    
    # Header
    printf "%-20s %-12s %s\n" "NAME" "STATUS" "SOURCE"
    printf "%-20s %-12s %s\n" "----" "------" "------"
    
    # Sort by name
    IFS=$'\n' sorted=($(sort <<< "${data_ref[*]}"))
    unset IFS
    
    for entry in "${sorted[@]}"; do
        local name="${entry%%:*}"
        local rest="${entry#*:}"
        local status="${rest%%:*}"
        local sources="${rest#*:}"
        
        # Replace | with comma for display
        sources="${sources//|/, }"
        
        printf "%-20s %-12s %s\n" "$name" "$status" "$sources"
    done
}

# Output list as JSON
output_json_list() {
    local -n data_ref=$1
    
    echo "["
    
    # Sort by name
    IFS=$'\n' sorted=($(sort <<< "${data_ref[*]}"))
    unset IFS
    
    local first=true
    for entry in "${sorted[@]}"; do
        local name="${entry%%:*}"
        local rest="${entry#*:}"
        local status="${rest%%:*}"
        local sources="${rest#*:}"
        
        if [[ "$first" == true ]]; then
            first=false
        else
            echo ","
        fi
        
        echo -n "  {"
        echo -n "\"name\": \"$name\", "
        echo -n "\"status\": \"$status\", "
        
        if [[ "$status" == "conflict" ]]; then
            # Output sources as array
            echo -n "\"sources\": ["
            local src_first=true
            IFS='|' read -ra src_array <<< "$sources"
            for src in "${src_array[@]}"; do
                if [[ "$src_first" == true ]]; then
                    src_first=false
                else
                    echo -n ", "
                fi
                echo -n "\"$src\""
            done
            echo -n "]"
        else
            echo -n "\"source\": \"$sources\""
        fi
        
        echo -n "}"
    done
    
    echo ""
    echo "]"
}

# Perform sync operation
perform_sync() {
    local -n roots_ref=$1
    local -n selected_ref=$2
    local dry_run="$3"
    
    # Create destination directory if needed
    if [[ "$dry_run" != true ]]; then
        mkdir -p "$DEST_DIR"
    fi
    
    # Build map of discovered skills to their unique source
    declare -A skill_to_source
    declare -A discovered_skills
    
    for root in "${roots_ref[@]}"; do
        for skill_dir in "$root"/*/; do
            skill_dir="${skill_dir%/}"
            [[ ! -d "$skill_dir" ]] && continue
            
            local skill_name
            skill_name="$(basename "$skill_dir")"
            
            if [[ -f "$skill_dir/SKILL.md" ]]; then
                discovered_skills[$skill_name]=1
                if [[ -z "${skill_to_source[$skill_name]:-}" ]]; then
                    skill_to_source[$skill_name]="$root"
                fi
            fi
        done
    done
    
    # Track which selected skills we successfully processed
    declare -A processed_skills
    
    # Process each selected skill
    for skill in "${selected_ref[@]}"; do
        if [[ -z "${discovered_skills[$skill]:-}" ]]; then
            warn "Selected skill not found in any source root: $skill"
            continue
        fi
        
        local source_root="${skill_to_source[$skill]}"
        local source_path="$source_root/$skill"
        local dest_path="$DEST_DIR/$skill"
        
        # Validate source has SKILL.md
        if [[ ! -f "$source_path/SKILL.md" ]]; then
            error "Selected skill directory missing SKILL.md: $source_path/$skill" 4
        fi
        
        # Check if destination exists
        if [[ -d "$dest_path" ]]; then
            if [[ "$dry_run" == true ]]; then
                echo "[DRY-RUN] UPDATE: $source_path -> $dest_path"
            else
                # Atomic replacement
                local temp_dest="$DEST_DIR/.tmp.$skill.$$"
                local backup_dest="$DEST_DIR/.backup.$skill.$$"
                
                # Copy to temp
                cp -r "$source_path" "$temp_dest"
                
                # Move existing to backup
                mv "$dest_path" "$backup_dest"
                
                # Move temp to destination
                mv "$temp_dest" "$dest_path"
                
                # Remove backup
                rm -rf "$backup_dest"
                
                echo "UPDATED: $skill"
            fi
        else
            if [[ "$dry_run" == true ]]; then
                echo "[DRY-RUN] COPY: $source_path -> $dest_path"
            else
                # Simple copy
                cp -r "$source_path" "$dest_path"
                echo "COPIED: $skill"
            fi
        fi
        
        processed_skills[$skill]=1
    done
    
    # Prune destination - remove skills not in selected list
    if [[ -d "$DEST_DIR" ]]; then
        for dest_skill_dir in "$DEST_DIR"/*/; do
            dest_skill_dir="${dest_skill_dir%/}"
            [[ ! -d "$dest_skill_dir" ]] && continue
            
            # Skip hidden directories (temp, backup)
            local skill_name
            skill_name="$(basename "$dest_skill_dir")"
            [[ "$skill_name" == .* ]] && continue
            
            # Check if this skill is in selected list
            local is_selected=false
            for sel in "${selected_ref[@]}"; do
                if [[ "$sel" == "$skill_name" ]]; then
                    is_selected=true
                    break
                fi
            done
            
            if [[ "$is_selected" == false ]]; then
                if [[ "$dry_run" == true ]]; then
                    echo "[DRY-RUN] DELETE: $dest_skill_dir"
                else
                    rm -rf "$dest_skill_dir"
                    echo "REMOVED: $skill_name"
                fi
            fi
        done
    fi
}

# Main function
main() {
    parse_args "$@"
    
    # Read configuration
    declare -a source_roots=()
    read_source_roots "$CONFIG_FILE" source_roots
    
    # Read selected skills
    declare -a selected_skills=()
    read_selected_skills "$SELECTED_FILE" selected_skills
    
    # Discover all skills
    declare -a discovered=()
    discover_skills source_roots discovered
    
    # Check for conflicts
    declare -a conflicts=()
    if ! check_conflicts discovered conflicts; then
        for conflict in "${conflicts[@]}"; do
            local name="${conflict%%:*}"
            local roots_str="${conflict#*:}"
            roots_str="${roots_str//|/\n  - }"
            error "Skill '$name' found in multiple source roots:\n  - $roots_str" 4
        done
    fi
    
    case "$MODE" in
        list)
            # Build list data
            declare -a list_data=()
            build_list_data discovered selected_skills list_data
            
            if [[ "$JSON_OUTPUT" == true ]]; then
                output_json_list list_data
            else
                output_human_list list_data
            fi
            ;;
        sync)
            perform_sync source_roots selected_skills "$DRY_RUN"
            ;;
    esac
}

main "$@"
