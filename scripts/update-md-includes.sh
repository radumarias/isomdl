#!/bin/bash

set -e

# Function to process each include block
process_include_blocks() {
    local readme_path=$1
    local temp_file=$2

    local start_marker
    local end_marker="\`\`\`"
    local include_file
    local include_content
    local inside_include_block=0
    local inside_code_block=0

      while IFS= read -r line || [ -n "$line" ]; do
        if [[ $inside_include_block -eq 0 && "$line" == *"<!-- INCLUDE-RUST: "* ]]; then
            start_marker="$line"
            include_file=$(echo "$line" | sed -n 's/.*<!-- INCLUDE-RUST: \(.*\) -->.*/\1/p')
            echo "Processing '$readme_path' include block for: $include_file"
            include_content=$(<"$include_file")
            echo "$start_marker" >> "$temp_file"
            inside_include_block=1
        elif [[ $inside_include_block -eq 1 && "$line" == '```rust' ]]; then
            echo "$line" >> "$temp_file"
            echo "$include_content" >> "$temp_file"
            inside_code_block=1
        elif [[ $inside_include_block -eq 1 && $inside_code_block -eq 1 && "$line" == "$end_marker" ]]; then
            echo "$line" >> "$temp_file"
            inside_include_block=0
            inside_code_block=0
        elif [[ $inside_include_block -eq 0 || $inside_code_block -eq 0 ]]; then
            echo "$line" >> "$temp_file"
        fi
    done < "$readme_path"
}

IN_FILE=$1
TEMP_FILE=$(mktemp)
# Call the function to process the include blocks
process_include_blocks "$IN_FILE" "$TEMP_FILE"

# Replace the original README file with the updated content, if content changed
# strip trailing spaces from lines starting with '//! '
temp_file2=$(mktemp)
sed -E 's/^(\/\/!?)[[:space:]]+$/\1/' "$TEMP_FILE" > "$temp_file2"
mv "$temp_file2" "$TEMP_FILE"
if ! cmp -s "$IN_FILE" "$TEMP_FILE"; then
    mv "$TEMP_FILE" "$IN_FILE"
else
    rm "$TEMP_FILE"
fi