#!/bin/bash

echo "Uninstall code..."
dfx canister uninstall-code prediction_markets_backend 

echo "Deploying code..."
dfx deploy prediction_markets_backend