#!/bin/bash

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}struct uninstaller${NC}"
echo ""

# Check if Linux
OS="$(uname -s)"
if [ "$OS" != "Linux" ]; then
    echo -e "${RED}This uninstaller only supports Linux${NC}"
    exit 1
fi

# Check common installation locations
LOCATIONS=(
    "/usr/local/bin/struct"
    "$HOME/.local/bin/struct"
    "/usr/bin/struct"
)

FOUND=()

echo -e "${CYAN}Checking for struct installations...${NC}"
echo ""

for location in "${LOCATIONS[@]}"; do
    if [ -f "$location" ]; then
        FOUND+=("$location")
        echo -e "${YELLOW}Found: $location${NC}"
    fi
done

if [ ${#FOUND[@]} -eq 0 ]; then
    echo -e "${YELLOW}No struct installations found${NC}"
    echo ""
    echo "Checked locations:"
    for location in "${LOCATIONS[@]}"; do
        echo "  - $location"
    done
    exit 0
fi

echo ""
echo -e "${CYAN}Select what to remove:${NC}"

# Show menu with found installations
for i in "${!FOUND[@]}"; do
    echo "  $((i+1))) ${FOUND[$i]}"
done
echo "  $((${#FOUND[@]}+1))) Remove all"
echo "  $((${#FOUND[@]}+2))) Cancel"

echo ""
read -p "Enter choice: " choice

# Handle all option
if [ "$choice" -eq $((${#FOUND[@]}+1)) ]; then
    echo ""
    for location in "${FOUND[@]}"; do
        if [[ "$location" == "/usr/local/bin/"* ]] || [[ "$location" == "/usr/bin/"* ]]; then
            echo -e "${YELLOW}Removing $location (requires sudo)${NC}"
            sudo rm -f "$location"
        else
            echo -e "${GREEN}Removing $location${NC}"
            rm -f "$location"
        fi
    done
    
    # Remove config if exists
    CONFIG_PATH="$HOME/.config/struct"
    if [ -d "$CONFIG_PATH" ]; then
        echo ""
        read -p "Also remove config directory ($CONFIG_PATH)? (y/n): " remove_config
        if [ "$remove_config" = "y" ] || [ "$remove_config" = "Y" ]; then
            rm -rf "$CONFIG_PATH"
            echo -e "${GREEN}Removed config directory${NC}"
        fi
    fi
    
    echo ""
    echo -e "${GREEN}All installations removed!${NC}"
    exit 0
fi

# Handle cancel
if [ "$choice" -eq $((${#FOUND[@]}+2)) ]; then
    echo ""
    echo -e "${YELLOW}Cancelled${NC}"
    exit 0
fi

# Handle single selection
if [ "$choice" -ge 1 ] && [ "$choice" -le ${#FOUND[@]} ]; then
    location="${FOUND[$((choice-1))]}"
    echo ""
    
    if [[ "$location" == "/usr/local/bin/"* ]] || [[ "$location" == "/usr/bin/"* ]]; then
        echo -e "${YELLOW}Removing $location (requires sudo)${NC}"
        sudo rm -f "$location"
    else
        echo -e "${GREEN}Removing $location${NC}"
        rm -f "$location"
    fi
    
    # Check if this was the last installation
    remaining=0
    for loc in "${FOUND[@]}"; do
        if [ -f "$loc" ] && [ "$loc" != "$location" ]; then
            remaining=$((remaining+1))
        fi
    done
    
    # Ask about config only if this was the last installation
    if [ $remaining -eq 0 ]; then
        CONFIG_PATH="$HOME/.config/struct"
        if [ -d "$CONFIG_PATH" ]; then
            echo ""
            read -p "Also remove config directory ($CONFIG_PATH)? (y/n): " remove_config
            if [ "$remove_config" = "y" ] || [ "$remove_config" = "Y" ]; then
                rm -rf "$CONFIG_PATH"
                echo -e "${GREEN}Removed config directory${NC}"
            fi
        fi
    fi
    
    echo ""
    echo -e "${GREEN}Uninstalled successfully!${NC}"
else
    echo ""
    echo -e "${RED}Invalid choice${NC}"
    exit 1
fi