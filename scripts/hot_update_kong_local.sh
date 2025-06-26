#!/usr/bin/env bash
# note: the way we resolve build kong script is via root dir, so make sure when quickly updating locally or staging
# happens via root dir, not the scripts dir

# Hot update script for Kong backend development
# This script:
# 1. Increments version (local/staging only)
# 2. Cleans build artifacts
# 3. Rebuilds kong_backend with proper environment
# 4. Optimizes and compresses for production (ic network)
# 5. Checks module hash before deployment (ic network)
# 6. Deploys with upgrade
# 7. Verifies the deployment

set -e

# Colors for output
if [ -t 1 ] && command -v tput >/dev/null && [ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]; then
    BOLD="$(tput bold)"
    NORMAL="$(tput sgr0)"
    GREEN="$(tput setaf 2)"
    BLUE="$(tput setaf 4)"
    RED="$(tput setaf 1)"
    YELLOW="$(tput setaf 3)"
else
    BOLD=""
    NORMAL=""
    GREEN=""
    BLUE=""
    RED=""
    YELLOW=""
fi

print_header() {
    echo
    echo "${BOLD}========== $1 ==========${NORMAL}"
    echo
}

print_success() {
    echo "${GREEN}✓${NORMAL} $1"
}

print_error() {
    echo "${RED}✗${NORMAL} $1" >&2
}

print_info() {
    echo "${BLUE}ℹ${NORMAL} $1"
}

print_warning() {
    echo "${YELLOW}⚠${NORMAL} $1"
}

# Network parameter (default to local)
NETWORK="${1:-local}"
NETWORK_FLAG=""
IDENTITY_FLAG=""
if [ "${NETWORK}" != "local" ]; then
    NETWORK_FLAG="--network ${NETWORK}"
fi

# Set identity for production networks
if [ "${NETWORK}" == "ic" ]; then
    IDENTITY_FLAG="--identity glad"
elif [ "${NETWORK}" == "staging" ]; then
    IDENTITY_FLAG="--identity kong"
fi

print_header "HOT UPDATE KONG BACKEND - $(echo ${NETWORK} | tr '[:lower:]' '[:upper:]')"

# Step 1: Version management (skip for production)
if [ "${NETWORK}" == "ic" ]; then
    print_warning "Production deployment - skipping version increment"
    print_info "Ensure version has been manually updated in Cargo.toml if needed"
else
    print_info "Getting current version..."
    CARGO_FILE="src/kong_backend/Cargo.toml"
    CURRENT_VERSION=$(grep '^version' $CARGO_FILE | sed 's/version = "\(.*\)"/\1/')
    print_info "Current version: $CURRENT_VERSION"

    # Increment patch version
    IFS='.' read -ra VERSION_PARTS <<< "$CURRENT_VERSION"
    MAJOR="${VERSION_PARTS[0]}"
    MINOR="${VERSION_PARTS[1]}"
    PATCH="${VERSION_PARTS[2]}"
    NEW_PATCH=$((PATCH + 1))
    NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"

    print_info "Updating to version: $NEW_VERSION"
    sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" $CARGO_FILE
fi

# Step 2: Clean build artifacts
print_header "CLEAN BUILD"
print_info "Running cargo clean..."
cargo clean -p kong_backend
print_success "Build artifacts cleaned"

# Step 3: Build kong_backend with proper environment
print_header "BUILD"
print_info "Building kong_backend for ${NETWORK}..."

if [ "${NETWORK}" == "ic" ]; then
    # Production build with optimizations
    print_info "Building with production features..."
    KONG_BUILDENV="ic" cargo build --features "prod" --target wasm32-unknown-unknown --release -p kong_backend --locked
    
    # Optimize and compress for production
    print_info "Optimizing WASM for production..."
    WASM_PATH=".dfx/ic/canisters/kong_backend/kong_backend.wasm"
    OPT_WASM_PATH=".dfx/ic/canisters/kong_backend/kong_backend_opt.wasm"
    
    if [ -f "$WASM_PATH" ]; then
        ic-wasm "$WASM_PATH" -o "$OPT_WASM_PATH" optimize O3
        gzip -c "$OPT_WASM_PATH" > "${WASM_PATH}.gz"
        rm "$OPT_WASM_PATH"
        print_success "WASM optimized and compressed"
    else
        print_error "WASM file not found at $WASM_PATH"
        exit 1
    fi
elif [ "${NETWORK}" == "staging" ]; then
    # Staging build
    KONG_BUILDENV="staging" cargo build --features "staging" --target wasm32-unknown-unknown --release -p kong_backend --locked
else
    # Local build
    ./scripts/build_kong_backend.sh local
fi

print_success "Build completed"

# Step 4: Pre-deployment verification for production
if [ "${NETWORK}" == "ic" ]; then
    print_header "PRE-DEPLOYMENT VERIFICATION"
    
    # Get current module hash
    print_info "Checking current module hash..."
    CURRENT_HASH=$(dfx canister info kong_backend ${NETWORK_FLAG} | grep "Module hash:" | awk '{print $3}' || echo "unknown")
    print_info "Current module hash: $CURRENT_HASH"
    
    # Calculate new module hash
    if [ -f ".dfx/ic/canisters/kong_backend/kong_backend.wasm.gz" ]; then
        NEW_HASH=$(sha256sum .dfx/ic/canisters/kong_backend/kong_backend.wasm.gz | awk '{print $1}')
        print_info "New module hash: $NEW_HASH"
        
        if [ "$CURRENT_HASH" == "$NEW_HASH" ]; then
            print_warning "Module hashes are identical - no changes detected"
            read -p "Continue with deployment anyway? (y/N): " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                print_info "Deployment cancelled"
                exit 0
            fi
        else
            print_success "Module hash verification passed - changes detected"
        fi
    else
        print_error "Compressed WASM file not found"
        exit 1
    fi
fi

# Step 5: Deploy with upgrade
print_header "DEPLOY"
print_info "Deploying kong_backend to ${NETWORK}..."

if [ "${NETWORK}" == "ic" ]; then
    # Production deployment - no stop/start, use upgrade-unchanged
    print_info "Performing production upgrade..."
    dfx deploy kong_backend ${NETWORK_FLAG} ${IDENTITY_FLAG} --upgrade-unchanged
else
    # Development deployment - stop/start for clean upgrade
    print_info "Stopping canister..."
    dfx canister stop kong_backend ${NETWORK_FLAG} || true

    # Deploy with upgrade
    dfx deploy kong_backend ${NETWORK_FLAG} ${IDENTITY_FLAG} --upgrade-unchanged

    # Start the canister again
    print_info "Starting canister..."
    dfx canister start kong_backend ${NETWORK_FLAG}
fi

print_success "Deployment completed"

# Step 6: Post-deployment verification
print_header "POST-DEPLOYMENT VERIFICATION"
print_info "Verifying deployment..."

# Call the canister to get version info
KONG_INFO=$(dfx canister call ${NETWORK_FLAG} kong_backend icrc1_name 2>&1 || echo "")
if [[ "$KONG_INFO" == *"Kong"* ]] || [[ "$KONG_INFO" == *"kong"* ]]; then
    print_success "Kong backend is responding"
    print_info "Response: $KONG_INFO"
else
    print_error "Kong backend verification failed"
    echo "Response: $KONG_INFO"
    exit 1
fi

# For production, verify module hash changed
if [ "${NETWORK}" == "ic" ]; then
    print_info "Verifying module hash update..."
    DEPLOYED_HASH=$(dfx canister info kong_backend ${NETWORK_FLAG} | grep "Module hash:" | awk '{print $3}' || echo "unknown")
    print_info "Deployed module hash: $DEPLOYED_HASH"
    
    if [ "$DEPLOYED_HASH" != "$CURRENT_HASH" ]; then
        print_success "Module hash updated successfully"
    else
        print_warning "Module hash unchanged - deployment may not have taken effect"
    fi
fi

# Summary
print_header "SUMMARY"
print_success "Hot update completed!"
if [ "${NETWORK}" != "ic" ]; then
    print_info "Version updated: $CURRENT_VERSION → $NEW_VERSION"
fi
print_info "Network: ${NETWORK}"
print_info "Kong backend canister ID: $(dfx canister id ${NETWORK_FLAG} kong_backend)"

if [ "${NETWORK}" == "ic" ]; then
    print_info "Production deployment completed with optimizations"
    print_warning "Monitor canister health and performance after deployment"
fi