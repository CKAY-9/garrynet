# Forum

Part of [GarryNet Design](DESIGN.md).

The forum is the community hub, where users can interact and post about any topic. The forum holds sections, sections hold categories, categories hold threads, and threads contain posts.

## Section

Sections should be rendered on the forum homepage, and should contain all of their categories. There should be clear seperation between sections.
Section headers should have the name, icon (if applicable), and accent color.
All of a section's categories are reachable from here.

```
id: number
name: string
accent: string // hex color
icon_url?: string // can be null/empty
description: string // max 256 characters
position: number // listed from highest to lowest; -1 = always last
created_at: utc timestamp
last_updated?: utc timestamp // empty/null if no changes since creation
```

## Category

Categories are collection of threads. Categories belong to sections. Locked categories
mean no new changes can happen except for unlocking it. 

```
id: number
name: string
section_id: number
description: string // max 256 characters
locked: boolean // prevents new threads from being created in it
position: number // same ordering rules as Section
created_at: utc timestamp
last_updated?: utc timestamp // empty/null if no changes since creation
```

## Thread

Deleting a thread deletes all of its posts; deleting a thread's first post deletes the thread.
Threads can be edited in the form of
- sticky
- locked
- allow templating
- title

```
id: number
author_id: number
category_id: number
title: string
thread_post_id: number // id of the thread's first post
created_at: utc timestamp
last_activity: utc timestamp
locked: boolean // prevents any changes to the thread
sticky: boolean // lists the thread first, alongside other stickies
allow_template: boolean // if set, users can copy this thread as a template for new threads
```

## Post

```
id: number
author_id: number
thread_id: number
content: string // Markdown, see below
created_at: utc timestamp
edited?: utc timestamp // null/empty if no edit applied
likes: number[] // list of ids who liked the post
dislikes: number[] // list of ids who disliked the post
attachments: { owner_id, id, name, content_type, size }[] // attached files, see below
```

### Attachments

A post can have up to 5 files attached (see [STORAGE.md](STORAGE.md)). The author uploads them first, then sends their file IDs as `attachments` when starting the thread or replying. Each must be a ready file in the author's own folder that isn't attached to anything else; otherwise 400. Attachments are set when the post is written and editing a post doesn't change them. Deleting a post or thread deletes the attached files.

## Routes

Reading is public. Writing needs a login; sections and categories also need Admin Access.

- GET `/forum`: every section (in display order) with its categories, each with `thread_count`, `post_count` and `latest` (the most recently active thread: `id`, `title`, `last_activity`, `author`), plus `stats` (`threads`, `posts`, `members`, `online`)
- POST `/forum/sections`: create a section (`name`, `accent`, `icon_url`, `description`, `position`). Requires Admin Access and Section Management
- DELETE `/forum/sections/{id}`: delete a section and everything in it. Requires Admin Access and Section Management
- POST `/forum/categories`: create a category (`name`, `section_id`, `description`, `locked`, `position`). Requires Admin Access and Category Management
- DELETE `/forum/categories/{id}`: delete a category and its threads. Requires Admin Access and Category Management
- GET `/forum/categories/{id}`: `{ category, section, threads }`. Threads list stickies first, then most recently active; each includes `author` (`id`, `username`, `avatar_url`) and `replies`
- POST `/forum/categories/{id}/threads`: start a thread (`title`, max 128 characters; `content`, the first post; optional `allow_template` and `locked`, both default `false`; optional `attachments`, up to 5 file IDs). Requires Site Access and User Posts; `locked: true` also needs Thread Management. 423 if the category is locked
- GET `/forum/threads/{id}`: `{ category, section, thread, posts }`. The thread's first post comes first, then replies oldest first; each post includes `author`
- POST `/forum/threads/{id}/posts`: reply (`content`, max 20,000 characters; optional `attachments`, up to 5 file IDs). Requires Site Access and User Posts; 423 if the thread or its category is locked. Bumps the thread's `last_activity`
- PATCH `/forum/threads/{id}`: change any of `title`, `locked`, `sticky`, `allow_template` (omitted fields are left alone). Authors can change `title` and `allow_template`; `locked` and `sticky` need Thread Management
- DELETE `/forum/threads/{id}`: delete a thread and all its posts
- PATCH `/forum/posts/{id}`: replace a post's `content` (same rules as replying) and set `edited`. Doesn't bump `last_activity` or change attachments
- DELETE `/forum/posts/{id}`: delete a post. Deleting a thread's first post deletes the thread; deleting a reply rewinds the thread's `last_activity` to its newest remaining post

### Managing threads and posts

The four routes above follow the same rules:

- Users with Site Access and Thread Management (moderators) can manage any thread or post, even in a locked thread or category.
- Otherwise users can only manage their own threads and posts, and only while they can still post (Site Access and User Posts, so not banned). 403 for someone else's.
- Authors can't change anything in a locked thread or category (423).

Post content is GitHub-flavored Markdown (tables, strikethrough, task lists, footnotes). The backend stores the Markdown source after sanitizing it (`src/routes/markdown.rs`): raw HTML is backslash-escaped so it shows as text, and a link or image whose URL isn't http(s), mailto or relative is a 400. The frontend renders it with react-markdown (`components/Markdown`), where single newlines are line breaks, raw HTML is skipped and unsafe URLs are dropped, as a second layer.

Names are at most 64 characters, and positions must be -1 or greater. A blank `accent` defaults to `#3b82f6`.

## Pages

### /forum

The homepage of the Forum. Just include all the different sections with their categories in each. Statistics like online users, total posts.

### /forum/{category_id}

Navigating here will reveal the category page which contains threads related to it. Users who can post get a "New thread" button (unless the category is locked); logged-out users are prompted to log in.

### /forum/{category_id}/{thread_id}

Thread page. Shows all posts in the thread, with the principal post/author being at the top, then a reply box for users who can post (unless the thread or category is locked). The new thread and reply forms have an "Attach files" button; attached images show as thumbnails under the post and other files as links. Users who can manage the thread get Rename and Delete thread buttons under the title (plus Lock and Sticky for moderators), and Edit and Delete buttons on each post they can manage. Deletes ask for confirmation inline. A thread opened under the wrong category id redirects to the right one.