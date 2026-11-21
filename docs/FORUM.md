# Forum

Part of [GravNet Design](README.md).

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

A post can have up to 5 files attached (see [STORAGE.md](STORAGE.md)). The author uploads them first, then sends their file IDs as `attachments` when starting the thread or replying. Each must be a ready file in the author's own folder that isn't attached to anything else; otherwise 400. Attachments are set when the post is written and editing a post doesn't change them. Deleting a post, thread, category or section deletes the attached files.

## Routes

Reading is public. Writing needs a login; sections and categories also need Admin Access.

- GET `/forum`: every section (in display order) with its categories, each with `thread_count`, `post_count` and `latest` (the most recently active thread: `id`, `title`, `last_activity`, `author`), plus `stats` (`threads`, `posts`, `members`, `online`)
- POST `/forum/sections`: create a section (`name`, `accent`, `icon_url`, `description`, `position`). Requires Admin Access and Section Management
- DELETE `/forum/sections/{id}`: delete a section and everything in it. Requires Admin Access and Section Management
- POST `/forum/categories`: create a category (`name`, `section_id`, `description`, `locked`, `position`). Requires Admin Access and Category Management
- DELETE `/forum/categories/{id}`: delete a category and its threads. Requires Admin Access and Category Management
- GET `/forum/categories/{id}?page=`: `{ category, section, threads, page, pages }`, 25 threads a page. Threads list stickies first, then most recently active; each includes `author` (`id`, `username`, `avatar_url`) and `replies`. `page` starts at 1 (the default); one past the end gives the last page, and the response's `page` says which was shown. `pages` is how many there are, at least 1. 400 for a `page` below 1
- POST `/forum/categories/{id}/threads`: start a thread (`title`, max 128 characters; `content`, the first post; optional `allow_template` and `locked`, both default `false`; optional `attachments`, up to 5 file IDs). Requires Site Access and User Posts; `locked: true` also needs Thread Management. 423 if the category is locked; 429 (with `Retry-After`) past 3 threads per 10 minutes, or past the post limit below, unless the user has Thread Management
- GET `/forum/threads/{id}?page=` or `?post=`: `{ category, section, thread, posts, page, pages }`, 20 posts a page, with `page` and `pages` as for categories. The thread's first post comes first (so only on page 1), then replies oldest first; each post includes `author`. `post={id}` shows the page holding that post instead, or page 1 if the thread has no such post (it may have been deleted)
- POST `/forum/threads/{id}/posts`: reply (`content`, max 20,000 characters; optional `attachments`, up to 5 file IDs). Requires Site Access and User Posts; 423 if the thread or its category is locked; 429 (with `Retry-After`) past 6 posts per minute, counting threads' first posts, unless the user has Thread Management. Bumps the thread's `last_activity`
- PATCH `/forum/threads/{id}`: change any of `title`, `locked`, `sticky`, `allow_template` (omitted fields are left alone). Authors can change `title` and `allow_template`; `locked` and `sticky` need Thread Management
- DELETE `/forum/threads/{id}`: delete a thread and all its posts
- PATCH `/forum/posts/{id}`: replace a post's `content` (same rules as replying) and set `edited`. Doesn't bump `last_activity` or change attachments
- PUT `/forum/posts/{id}/vote`: `{ vote: "like" | "dislike" | null }`. Sets your vote on the post and returns the post; `null` takes it back, and a like replaces a dislike (and vice versa). Repeating a vote changes nothing. Requires Site Access and User Posts; 400 for your own post, 423 if the thread or its category is locked. See [Reputation](#reputation)
- DELETE `/forum/posts/{id}`: delete a post. Deleting a thread's first post deletes the thread; deleting a reply rewinds the thread's `last_activity` to its newest remaining post

### Managing threads and posts

The four routes above follow the same rules:

- Users with Site Access and Thread Management (moderators) can manage any thread or post, even in a locked thread or category, except those written by a user holding permissions they don't (403), so a moderator can't edit or delete an admin's posts. The author's usergroups count, punishments aside, as for bans.
- Otherwise users can only manage their own threads and posts, and only while they can still post (Site Access and User Posts, so not banned). 403 for someone else's.
- Authors can't change anything in a locked thread or category (423).

### Reputation

A user's reputation is the total likes minus the total dislikes across all of their posts (shown on their profile; see [USERS.md](USERS.md)). It's stored in `users.reputation` and kept up to date by a database trigger on `forum_posts`, so it changes with every vote and drops a post's likes and dislikes when the post goes, including when its thread, category or section is deleted. Votes from users who are later banned or suspended still count.

Post content is GitHub-flavored Markdown (tables, strikethrough, task lists, footnotes). The backend stores the Markdown source after sanitizing it (`src/routes/markdown.rs`): raw HTML is backslash-escaped so it shows as text, and a link or image whose URL isn't http(s), mailto or relative is a 400. The frontend renders it with react-markdown (`components/Markdown`), where single newlines are line breaks, raw HTML is skipped and unsafe URLs are dropped, as a second layer.

Names are at most 64 characters, and positions must be -1 or greater. A blank `accent` defaults to `#3b82f6`.

## Pages

### /forum

The homepage of the Forum. Just include all the different sections with their categories in each. Statistics like online users, total posts.

### /forum/{category_id}

Navigating here will reveal the category page which contains threads related to it, 25 a page (`?page=`). Users who can post get a "New thread" button (unless the category is locked); logged-out users are prompted to log in.

### /forum/{category_id}/{thread_id}

Thread page. Shows the thread's posts 20 a page (`?page=`), with the principal post/author at the top of the first, and links to the other pages above and below. `?post={id}` opens the page holding that post; replying, a reply notification, and a reply on a user's profile all link there, scrolled to the post (`#post-{id}`). Under the posts is a reply box for users who can post (unless the thread or category is locked). The new thread and reply forms have an "Attach files" button; attached images show as thumbnails under the post and other files as links. Users who can manage the thread get Rename and Delete thread buttons under the title (plus Lock and Sticky for moderators), and Edit and Delete buttons on each post they can manage. Each post shows its like and dislike counts; users who can post can click them to like or dislike other users' posts (clicking their current vote again takes it back), except in a locked thread or category. Deletes ask for confirmation inline. A thread opened under the wrong category id redirects to the right one.