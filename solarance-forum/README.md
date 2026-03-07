# Solarance Forum Module (MVP)

The `solarance-forum` module is a library designed to be imported by other SpacetimeDB projects to provide drop-in forum capabilities and basic moderation tools.

## MVP Scope

This module does not implement a full social media platform, but rather the bare essentials needed to host, organize, and moderate discussions within a SpacetimeDB application. Posts should be displayed as read if their latest posts' timestamp is after the user's "mark_all_as_read" timestamp - this will keep things as simple as possible while still allowing users to notice when new posts are made.

### Core Entities (Tables)

We envision the following minimal set of tables (`#[spacetimedb(table)]`):

1. **`Category`**: High-level organizational buckets for discussions (e.g., "Announcements", "General", "Help").
2. **`Thread`**: A specific topic inside a Category, created by a user, which holds multiple posts.
3. **`Post`**: The individual replies (or the initial message) inside a Thread.
4. **`ForumUser`**: A basic user profile tied to a SpacetimeDB Identity, optionally including a display name, signature, role (e.g., `Admin`, `Moderator`, `User`, or `Banned`), timestamp for last "Mark All As Read", and a `Vec` of their group IDs.
5. **`ModerationLog`**: A record of actions taken by moderators (e.g., who banned whom and why).

#### Enums

Categories can be:
```rust
#[derive(SpacetimeType, Clone, Debug, PartialEq)]
enum CategoryVisibility {
    Public,
    Group(u64),
    Moderator_Only
}
```

Forum roles can be:
```rust
#[derive(SpacetimeType, Clone, Debug, PartialEq)]
enum ForumRole {
    /// Has ultimate access
    Owner,
    /// Can do everything except delete the forum
    Admin,
    /// Can do everything except delete the forum or change roles - including posting in locked threads
    Moderator,
    /// Can view all categories, threads, and moderation log
    Auditor,
    /// Can view public categories and threads, and can post in any group that the user is a member of.
    User,
    /// Can only view public categories and threads, and cannot post.
    Banned,
}
```

### Key Reducers (Actions)

To manipulate the state, the module will expose reducers such as:

*   **User Actions**:
    *   `register_user(display_name: String)`: Set up a forum profile.
    *   `set_signature(signature: String)`: Set your signature.
    *   `create_thread(category_id: u64, title: String, content: String)`: Create a new discussion.
    *   `create_post(thread_id: u64, content: String)`: Reply to a thread that isn't locked.
    *   `reply_to_post(post_id: u64, content: String)`: Reply to a post in the same thread.
    *   `edit_own_post(post_id: u64, new_content: String)`: Edit your own post.
    *   `edit_own_thread(post_id: u64, new_title: String, new_content: String)`: Edit your own thread.
    *   `mark_all_as_read()`: Updates the user's read status for all posts in all categories.

*   **Admin Actions**:
    *   `create_category(name: String, description: String, visibility: CategoryVisibility)`: Creates a category visible only to members of a specific group.
    *   `edit_category(category_id: u64, name: String, description: String, visibility: CategoryVisibility, order_override: Option<u32>)`: Edits a category.
    *   `delete_category(category_id: u64, cascade: bool)`: Deletes a category.
    *   `set_category_visibility(category_id: u64, visibility: CategoryVisibility)`: Changes a category's visibility.

*   **Moderation Actions**:
    *   `edit_post(post_id: u64, new_content: String, reason: String)`: Edit any post.
    *   `delete_post(post_id: u64, reason: String)`: Soft delete a post (hides content, keeping record).
    *   `edit_thread(thread_id: u64, title: String, content: String, reason: String)`: Edit any thread.
    *   `pin_thread(thread_id: u64)`: Pin a thread to the top of the category.
    *   `lock_thread(thread_id: u64, reason: String)`: Prevent further replies.
    *   `unlock_thread(thread_id: u64, reason: String)`: Allow further replies.
    *   `lock_category(thread_id: u64, reason: String)`: Prevent further threads and replies.
    *   `unlock_category(thread_id: u64, reason: String)`: Allow further threads and replies.
    *   `ban_user(user_identity: Identity, reason: String)`: Prevent a user from posting.
    *   `unban_user(user_identity: Identity, reason: String)`: Allow a user to post again.
    *   `move_thread(thread_id: u64, new_category_id: u64)`: Move a thread to a different category.
    *   `move_post(post_id: u64, new_thread_id: u64)`: Move a post to a different thread.
    *   `set_user_display_name(user_identity: Identity, display_name: String, reason: String)`: Set a user's display name.
    *   `set_user_signature(user_identity: Identity, signature: String, reason: String)`: Set a user's signature.
    *   `set_user_role(user_identity: Identity, role: ForumRole, reason: String)`: Set a user's role.

### Key Views (Queries)

*   `get_categories()`: Returns all categories that are public or that the user has access to.
*   `get_threads(category_id: u64)`: Returns all threads in a category that are public or that the user has access to.
*   `get_posts(thread_id: u64)`: Returns all posts in a thread that are public or that the user has access to.
*   `get_moderation_log()`: Returns all moderation actions. Only for admins and moderators.
*   `get_my_profile()`: Returns the current user's profile.
*   `get_my_groups()`: Returns all groups that the current user is a member of.

## Usage

Other SpacetimeDB projects would include `solarance-forum` in their `Cargo.toml`. They can then leverage the exported reducers and tables directly, or wrap them in their own logic for fine-grained control.
