#!/bin/bash

cargo build --release

wasm-pack build --target web --out-dir docs --release

rm docs/.gitignore
rm docs/*.d.ts
rm docs/package.json

cp src/index.html docs


# Define file paths
readmeFile="Readme.md"
htmlFile="docs/index.html"
# Check if the sed command is GNU sed or BSD sed (macOS)
if sed --version 2>/dev/null | grep -q GNU; then
    # GNU sed
    sedExtendedRegexFlag="-r"
else
    # BSD sed (macOS)
    sedExtendedRegexFlag="-E"
fi

# Escape special characters in the Markdown content for sed
# This handles: &, \, /, newline. Extend if more characters need to be handled.
escapedContent=$(<"$readmeFile" sed -e 's/[&/\]/\\&/g' -e ':a' -e 'N' -e '$!ba' -e 's/\n/\\n/g')

# Replace the placeholder in the HTML file with the Markdown content
sed -i.bak $sedExtendedRegexFlag "s|<Readme.md placeholder>|$escapedContent|g" "$htmlFile"

# Optionally, remove the backup file created by sed (uncomment the next line to enable)
rm "${htmlFile}.bak"



cp target/release/obm docs/obm
