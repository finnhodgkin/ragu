#!/bin/bash

# Generate a large workspace with 2,000 packages for benchmarking
# This creates a realistic PureScript workspace structure

set -e

WORKSPACE_DIR="workspace-benchmark"
NUM_PACKAGES=2000

echo "Generating workspace benchmark with $NUM_PACKAGES packages..."

# Clean up any existing benchmark directory
if [ -d "$WORKSPACE_DIR" ]; then
    echo "Removing existing benchmark directory..."
    rm -rf "$WORKSPACE_DIR"
fi

# Create workspace structure
mkdir -p "$WORKSPACE_DIR/packages"

echo "Creating $NUM_PACKAGES packages..."

# Generate package names
PACKAGE_NAMES=()
for i in $(seq 0 $((NUM_PACKAGES-1))); do
    PACKAGE_NAMES+=("package-$(printf "%04d" $i)")
done

# Create packages with random dependencies
for i in $(seq 0 $((NUM_PACKAGES-1))); do
    PACKAGE_NAME="package-$(printf "%04d" $i)"
    PACKAGE_DIR="$WORKSPACE_DIR/packages/$PACKAGE_NAME"

    echo "Creating package $PACKAGE_NAME..."

    # Create package directory structure
    mkdir -p "$PACKAGE_DIR/src"

    # Create a simple PureScript file
    cat > "$PACKAGE_DIR/src/Main.purs" << EOF
module Package$i where

import Prelude

main :: Unit -> Unit
main _ = unit
EOF

    # Generate random dependencies (0-5 dependencies per package)
    # Only allow dependencies on packages with lower indices to prevent circular deps
    NUM_DEPS=$((RANDOM % 6))
    DEPS=("prelude")  # All packages depend on prelude

    if [ $i -gt 0 ]; then  # Only create dependencies if this isn't the first package
        for j in $(seq 1 $NUM_DEPS); do
            # Pick a random package with lower index (0 to i-1)
            DEP_INDEX=$((RANDOM % i))
            DEP_NAME="package-$(printf "%04d" $DEP_INDEX)"
            DEPS+=("$DEP_NAME")
        done

        # Remove duplicates from DEPS array
        DEPS=($(printf '%s\n' "${DEPS[@]}" | sort -u))
    fi

    # Create spago.yaml for this package
    cat > "$PACKAGE_DIR/spago.yaml" << EOF
package:
  name: $PACKAGE_NAME
  dependencies:
$(if [ ${#DEPS[@]} -eq 0 ]; then echo "    []"; else printf "    - %s\n" "${DEPS[@]}"; fi)
EOF
done

# Create root workspace spago.yaml
echo "Creating root workspace configuration..."

cat > "$WORKSPACE_DIR/spago.yaml" << EOF
package:
  name: workspace-root
  dependencies:
    - effect
    - prelude
    - console
  test:
    main: Test.Main
    dependencies: []

workspace:
  packageSet:
    url: https://raw.githubusercontent.com/purescript/package-sets/psc-0.15.15-20251004/packages.json
EOF

# Create a simple test file
mkdir -p "$WORKSPACE_DIR/test"
cat > "$WORKSPACE_DIR/test/Main.purs" << EOF
module Test.Main where

import Prelude
import Effect (Effect)
import Console (log)

main :: Effect Unit
main = do
  log "Workspace benchmark test"
EOF

echo ""
echo "Workspace benchmark generated successfully!"
echo "Directory: $WORKSPACE_DIR"
echo "Packages created: $NUM_PACKAGES"
echo ""
echo "To test your spago-rust tool:"
echo "  cd $WORKSPACE_DIR"
echo "  # Run your spago-rust commands here"
echo ""
echo "To clean up:"
echo "  rm -rf $WORKSPACE_DIR"
