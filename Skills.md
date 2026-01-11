# UHM - Universal Health Manager

## Project Overview

UHM is a health and nutrition tracking system built as an MCP (Model Context Protocol) server in Rust. It enables AI assistants like Claude to help users track their food intake, recipes, daily meals, and nutritional information through natural conversation.

## What We've Accomplished

### Phase 1: Foundation
- Created Rust project with MCP server using the `rmcp` crate
- Implemented auto-incrementing build number system
- Built `uhm_status` tool for querying service status (build info, uptime, memory usage, database info)
- Startup banner displaying version and build information

### Phase 2: Database
- SQLite database with r2d2 connection pooling
- WAL mode for better concurrent access
- Full schema with migrations support:
  - `food_items` - Base nutritional data
  - `recipes` - Recipe definitions with cached nutrition
  - `recipe_ingredients` - Junction table linking food items to recipes
  - `days` - Daily containers with cached nutrition totals
  - `meal_entries` - Logged food consumption
  - `vitals` - Health measurements (schema ready, tools pending)

### Phase 3: Food Item Tools
- `add_food_item` - Create food items with full nutritional data
- `search_food_items` - Search by name or brand
- `get_food_item` - Get detailed food item info with recipe usage count
- `list_food_items` - List with filtering, sorting, pagination
- `update_food_item` - Update food item (auto-recalculates nutrition for recipes using it)

### Phase 4: Recipe Tools
- `create_recipe` - Create recipes (ingredients added separately)
- `get_recipe` - Get full recipe with ingredients and calculated nutrition
- `list_recipes` - List with search, favorites filter, sorting
- `update_recipe` - Update metadata (blocked if logged in meals)
- `add_recipe_ingredient` - Add food items to recipes
- `update_recipe_ingredient` - Modify ingredient quantities
- `remove_recipe_ingredient` - Remove ingredients
- `recalculate_recipe_nutrition` - Force nutrition recalculation

### Phase 5: Day & Meal Entry Tools
- `get_or_create_day` - Get or create a day by date
- `get_day` - Full day view with meals organized by type
- `list_days` - List days with date range filtering
- `update_day` - Update day notes
- `log_meal` - Log food item or recipe consumption
- `get_meal_entry` - Get meal entry details
- `update_meal_entry` - Update servings, percent eaten, etc.
- `delete_meal_entry` - Remove meal entries
- `recalculate_day_nutrition` - Force daily totals recalculation

### Phase 6: AI Assistant Support
- `meal_instructions` - Returns step-by-step guide for logging meals (for AI assistants to reference)

## Technology Stack

### Rust
- **Why Rust**: Memory safety, performance, excellent error handling, strong type system
- **Async Runtime**: Tokio for async I/O
- **Database**: rusqlite with r2d2 connection pooling
- **Serialization**: serde + serde_json
- **Schema Generation**: schemars for JSON Schema from Rust types

### MCP (Model Context Protocol)
- **Crate**: `rmcp` v0.8
- **Transport**: stdio (for Claude Desktop integration)
- **Tools**: Defined using `#[tool]` macro with automatic schema generation
- **Server Handler**: Implements `ServerHandler` trait for MCP protocol

### Build System
- **Cargo**: Standard Rust build tool
- **build.rs**: Custom build script for compile-time code generation

## Build Number System

The build number automatically increments **only when source files change**, not on every `cargo build`.

### How It Works

1. **build.rs** script runs before compilation
2. Uses `cargo:rerun-if-changed=src` directive - only triggers on src/ changes
3. Reads current build number from `build_number.txt`
4. Increments and writes back the new number
5. Sets environment variables for compile-time embedding:
   - `UHM_BUILD_NUMBER` - The incremented build number
   - `UHM_BUILD_TIMESTAMP` - ISO 8601 datetime of compilation

### Files Involved
- `build.rs` - Build script with increment logic
- `build_number.txt` - Persistent counter file
- `src/build_info.rs` - Compile-time constants using `env!()` macro

### Current Build
Build number starts at 0 and increments to 1 on first build. Each source change triggers a new build number.

## Lessons Learned

### rmcp Crate API
- `Error` type was deprecated in favor of `ErrorData`
- `ServerInfo` structure requires specific fields: `protocol_version`, `capabilities`, `server_info`, `instructions`
- `Implementation` struct needs `title`, `icons`, `website_url` fields (can be None/Some)
- Use `#[tool_router]` on impl block and `#[tool]` on individual methods

### sysinfo Crate
- API changed: `refresh_process()` became `refresh_processes()` with `ProcessesToUpdate` enum
- Must use `ProcessesToUpdate::Some(&[Pid::from_u32(pid)])` for single process refresh

### Rust Patterns
- Connection pooling with r2d2 simplifies database access
- Using `Box<dyn ToSql>` for dynamic SQL parameter building
- Cached nutrition values avoid expensive recalculations on every query
- Data integrity rules: block updates to items used in dependent records

### MCP Integration
- Claude Desktop config at `%APPDATA%\Claude\claude_desktop_config.json`
- Server runs as stdio subprocess
- Tools appear automatically in Claude's interface after restart

## Project Structure

```
D:\Projects\UHM\
├── Cargo.toml              # Dependencies and project config
├── build.rs                # Build number auto-increment
├── build_number.txt        # Persistent build counter
├── Skills.md               # This document
├── UHM_DESIGN.md           # Original design specification
├── src/
│   ├── main.rs             # Entry point, startup banner, MCP server init
│   ├── build_info.rs       # Compile-time constants
│   ├── db/
│   │   ├── mod.rs
│   │   ├── connection.rs   # Database pool management
│   │   └── migrations.rs   # Schema creation
│   ├── models/
│   │   ├── mod.rs
│   │   ├── nutrition.rs    # Shared Nutrition struct
│   │   ├── food_item.rs    # FoodItem CRUD
│   │   ├── recipe.rs       # Recipe CRUD
│   │   ├── recipe_ingredient.rs  # Recipe ingredients + nutrition calc
│   │   ├── day.rs          # Day CRUD
│   │   └── meal_entry.rs   # MealEntry CRUD + day nutrition calc
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── status.rs       # uhm_status implementation
│   │   ├── food_items.rs   # Food item tool functions
│   │   ├── recipes.rs      # Recipe tool functions
│   │   └── days.rs         # Day and meal entry tool functions
│   └── mcp/
│       ├── mod.rs
│       └── server.rs       # MCP server, all tool definitions
└── data/
    └── uhm.db              # SQLite database (created on first run)
```

## Todo / Future Work

### Pending Implementation
- [ ] Vitals tracking tools (weight, blood pressure, heart rate, oxygen, glucose)
- [ ] Unit conversion utilities (grams ↔ oz, ml ↔ cups, etc.)
- [ ] Nutrition goals and daily targets
- [ ] Weekly/monthly nutrition reports
- [ ] Food item import from external databases (USDA, etc.)

### Potential Enhancements
- [ ] Barcode scanning integration
- [ ] Meal planning and suggestions
- [ ] Recipe scaling (adjust servings)
- [ ] Ingredient substitution suggestions
- [ ] Export data to CSV/JSON
- [ ] Web dashboard for visualization

## Running UHM

### Development
```bash
cargo run
```

### Release Build
```bash
cargo build --release
```

### Claude Desktop Integration
Add to `%APPDATA%\Claude\claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "uhm": {
      "command": "D:\\Projects\\UHM\\target\\release\\uhm.exe",
      "args": []
    }
  }
}
```

Restart Claude Desktop to load the UHM server.
