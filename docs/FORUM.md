# Forum

Part of [GarryNet Design](DESIGN.md).

The forum is the community hub, where users can interact and post about any topic. The forum holds sections, sections hold categories, categories hold threads, and threads contain posts.

## Section

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
content: string
created_at: utc timestamp
edited?: utc timestamp // null/empty if no edit applied
```

## Pages

### /forum

The homepage of the Forum. Just include all the different sections with their categories in each. Statistics like online users, total posts.

### /forum/{category_id}

Navigating here will reveal the category page which contains threads related to it.

### /forum/{category_id}/{thread_id}

Thread page. Shows all posts in the thread, with the principal post/author being at the top.