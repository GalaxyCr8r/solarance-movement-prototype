# Solarance Forum Module Design

This document details the software design for the `solarance-forum` SpacetimeDB backend module. This module acts as an add-on, meant to be imported into another SpacetimeDB module (like `spacetimedb` representing the main game server) to provide drop-in forum functionality.

## Project Structure

The source code will be organized to separate concerns, similar to the main game module:

```
solarance-forum/
├── Cargo.toml
├── README.md
├── design.md
└── src/
    ├── lib.rs          # Module entry point, exports elements
    ├── types.rs        # Enums and View structures (SpacetimeType)
    ├── tables.rs       # SpacetimeDB tables with SpacetimeDSL definitions
    ├── reducers.rs     # Modifying actions (User, Admin, Moderation)
    └── views.rs        # Read-only queries for the client
```

## Domain Model (Types)

Located in `src/types.rs`. Contains custom types and structures to avoid N+1 queries.

```rust
use spacetimedb::{SpacetimeType, Timestamp};
use crate::tables::{Category, Thread, Post, ForumUser};

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub enum CategoryVisibility {
    Public,
    Group(u64),
    ModeratorOnly,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub enum ForumRole {
    Owner,
    Admin,
    Moderator,
    Auditor,
    User,
    Banned,
}

// Derived Entities for Views
#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct CategoryPreview {
    pub category: Category,
    pub thread_count: u32,
    pub latest_post_timestamp: Option<Timestamp>,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ThreadPreview {
    pub thread: Thread,
    pub author_display_name: String,
    pub reply_count: u32,
    pub latest_post_timestamp: Option<Timestamp>,
    pub is_unread: bool,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct PostWithAuthor {
    pub post: Post,
    pub author: ForumUser,
}
```

## Table Definitions

Located in `src/tables.rs`. We use `spacetimedsl` macros to generate type-safe wrappers and accessors.

```rust
use spacetimedb::{table, Identity, Timestamp};
use spacetimedsl::dsl;
use crate::types::{CategoryVisibility, ForumRole};

#[dsl(plural_name = forum_users, method(update = true, delete = false))]
#[table(accessor = forum_user, public)]
pub struct ForumUser {
    #[primary_key]
    #[create_wrapper]
    pub id: Identity,
    
    #[index(btree)]
    #[unique]
    pub display_name: String,
    
    pub signature: String,
    pub role: ForumRole,
    pub mark_all_as_read: Timestamp,
    pub group_ids: Vec<u64>,

    pub created_at: Timestamp, 
    pub modified_at: Option<Timestamp>,
}

#[dsl(plural_name = categories, method(update = true, delete = true))]
#[table(accessor = category, public)]
pub struct Category {
    #[primary_key]
    #[create_wrapper]
    #[auto_inc]
    pub id: u64,
    
    pub name: String,
    pub description: String,
    pub visibility: CategoryVisibility,
    pub order: u32,

    pub created_at: Timestamp, 
    pub modified_at: Option<Timestamp>,
}

#[dsl(plural_name = threads, method(update = true, delete = true))]
#[table(accessor = thread, public)]
pub struct Thread {
    #[primary_key]
    #[create_wrapper]
    #[auto_inc]
    pub id: u64,
    
    #[index(btree)]
    #[use_wrapper(crate::CategoryId)]
    pub category_id: u64,
    
    #[index(btree)]
    #[use_wrapper(crate::ForumUserId)]
    pub author_id: Identity,
    
    pub title: String,
    pub content: String,
    pub is_pinned: bool,
    pub is_locked: bool,

    pub created_at: Timestamp, 
    pub modified_at: Option<Timestamp>,
}

#[dsl(plural_name = posts, method(update = true, delete = true))]
#[table(accessor = post, public)]
pub struct Post {
    #[primary_key]
    #[create_wrapper]
    #[auto_inc]
    pub id: u64,
    
    #[index(btree)]
    #[use_wrapper(crate::ThreadId)]
    #[foreign_key(path = self, table = thread, column = id, on_delete = Delete)]
    pub thread_id: u64,
    
    #[index(btree)]
    #[use_wrapper(crate::ForumUserId)]
    #[foreign_key(path = self, table = forum_user, column = id, on_delete = Ignore)]
    pub author_id: Identity,
    
    pub content: String,

    pub created_at: Timestamp, 
    pub modified_at: Option<Timestamp>,
    
    /// Optional reference to a specific post being replied to
    #[use_wrapper(crate::PostId)]
    pub reply_to_post_id: Option<u64>, 
}

#[dsl(plural_name = moderation_logs, method(update = false, delete = false))]
#[table(accessor = moderation_log)]
pub struct ModerationLog {
    #[primary_key]
    #[create_wrapper]
    #[auto_inc]
    pub id: u64,
    
    #[index(btree)]
    pub moderator_id: Identity,
    
    /// Identity of user acted upon, if applicable
    #[index(btree)]
    pub target_user_id: Option<Identity>,
    
    pub action: String,
    pub reason: String,

    pub created_at: Timestamp, 
}
```

## Reducer Interface

Located in `src/reducers.rs`.

```rust
use spacetimedb::{reducer, ReducerContext, Identity};
use crate::types::{CategoryVisibility, ForumRole};

// User Actions
#[reducer] pub fn register_user(ctx: &ReducerContext, display_name: String) -> Result<(), String>;
#[reducer] pub fn set_signature(ctx: &ReducerContext, signature: String) -> Result<(), String>;
#[reducer] pub fn create_thread(ctx: &ReducerContext, category_id: u64, title: String, content: String) -> Result<(), String>;
#[reducer] pub fn create_post(ctx: &ReducerContext, thread_id: u64, content: String) -> Result<(), String>;
#[reducer] pub fn reply_to_post(ctx: &ReducerContext, post_id: u64, content: String) -> Result<(), String>;
#[reducer] pub fn edit_own_post(ctx: &ReducerContext, post_id: u64, new_content: String) -> Result<(), String>;
#[reducer] pub fn edit_own_thread(ctx: &ReducerContext, thread_id: u64, new_title: String, new_content: String) -> Result<(), String>;
#[reducer] pub fn mark_all_as_read(ctx: &ReducerContext) -> Result<(), String>;

// Admin Actions
#[reducer] pub fn create_category(ctx: &ReducerContext, name: String, description: String, visibility: CategoryVisibility) -> Result<(), String>;
#[reducer] pub fn edit_category(ctx: &ReducerContext, category_id: u64, name: String, description: String, visibility: CategoryVisibility, order_override: Option<u32>) -> Result<(), String>;
#[reducer] pub fn delete_category(ctx: &ReducerContext, category_id: u64, cascade: bool) -> Result<(), String>;
#[reducer] pub fn set_category_visibility(ctx: &ReducerContext, category_id: u64, visibility: CategoryVisibility) -> Result<(), String>;

// Moderation Actions
#[reducer] pub fn edit_post(ctx: &ReducerContext, post_id: u64, new_content: String, reason: String) -> Result<(), String>;
#[reducer] pub fn delete_post(ctx: &ReducerContext, post_id: u64, reason: String) -> Result<(), String>;
#[reducer] pub fn edit_thread(ctx: &ReducerContext, thread_id: u64, title: String, content: String, reason: String) -> Result<(), String>;
#[reducer] pub fn pin_thread(ctx: &ReducerContext, thread_id: u64) -> Result<(), String>;
#[reducer] pub fn lock_thread(ctx: &ReducerContext, thread_id: u64, reason: String) -> Result<(), String>;
#[reducer] pub fn unlock_thread(ctx: &ReducerContext, thread_id: u64, reason: String) -> Result<(), String>;
#[reducer] pub fn ban_user(ctx: &ReducerContext, user_identity: Identity, reason: String) -> Result<(), String>;
#[reducer] pub fn unban_user(ctx: &ReducerContext, user_identity: Identity, reason: String) -> Result<(), String>;
#[reducer] pub fn move_thread(ctx: &ReducerContext, thread_id: u64, new_category_id: u64) -> Result<(), String>;
#[reducer] pub fn move_post(ctx: &ReducerContext, post_id: u64, new_thread_id: u64) -> Result<(), String>;
#[reducer] pub fn set_user_role(ctx: &ReducerContext, user_identity: Identity, role: ForumRole, reason: String) -> Result<(), String>;
```

## View Interface

Located in `src/views.rs`. These functions define queries that return aggregated data to the clients.

```rust
use spacetimedb::{view, ViewContext};
use crate::types::{CategoryPreview, ThreadPreview, PostWithAuthor, ForumUser};
use crate::tables::ModerationLog;

// Authorized Views (Returns subset of data user is allowed to see)
// Views can only accept `ViewContext` or `AnonymousViewContext` as the ONLY argument.

#[view(accessor = categories)]
pub fn get_all_categories(ctx: &ViewContext) -> Vec<CategoryPreview>;

/// Get all threads the current user can see.
#[view(accessor = threads)]
pub fn get_all_threads(ctx: &ViewContext) -> Vec<ThreadPreview>;

/// Get all posts in a thread the current user can see.
#[view(accessor = posts)]
pub fn get_all_posts(ctx: &ViewContext, thread_id: u64) -> Vec<PostWithAuthor>;

/// Get all moderation logs the current user can see. ONLY for moderators, auditor, and admins.
#[view(accessor = moderation_logs)]
pub fn get_all_moderation_logs(ctx: &ViewContext) -> Vec<ModerationLog>;

/// Get the current user's profile. Has to use Vec instead of Option for STDB implementation reasons.
#[view(accessor = my_profile)]
pub fn get_my_profile(ctx: &ViewContext) -> Vec<ForumUser>;

/// Get the current user's groups.
pub fn get_my_groups(ctx: &ViewContext) -> Vec<u64>;

// Anonymous/Public Views (Returns only public data)
#[view(accessor = public_categories)]
pub fn get_public_categories(ctx: &AnonymousViewContext) -> Vec<CategoryPreview>;

#[view(accessor = public_threads)]
pub fn get_public_threads(ctx: &AnonymousViewContext, category_id: u64) -> Vec<ThreadPreview>;

#[view(accessor = public_posts)]
pub fn get_public_posts(ctx: &AnonymousViewContext, thread_id: u64) -> Vec<PostWithAuthor>;
```
