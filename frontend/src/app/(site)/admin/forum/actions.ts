"use server";

import { refresh } from "next/cache";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { ForumCategory, ForumCategoryDetails, ForumSection, ForumSectionDetails } from "@/lib/types";

/** Creates a forum section. The backend validates it and checks the permissions. */
export async function createSection(details: ForumSectionDetails): Promise<ActionResult<ForumSection>> {
  return runAction("create the section", async () => {
    const { data } = await backend.post<ForumSection>("/forum/sections", details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** Deletes a section with every category, thread and post in it. */
export async function deleteSection(id: number): Promise<ActionResult<void>> {
  return runAction("delete the section", async () => {
    await backend.delete(`/forum/sections/${id}`, { headers: await authHeaders() });
    refresh();
  });
}

/** Creates a forum category in an existing section. */
export async function createCategory(details: ForumCategoryDetails): Promise<ActionResult<ForumCategory>> {
  return runAction("create the category", async () => {
    const { data } = await backend.post<ForumCategory>("/forum/categories", details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** Deletes a category with every thread and post in it. */
export async function deleteCategory(id: number): Promise<ActionResult<void>> {
  return runAction("delete the category", async () => {
    await backend.delete(`/forum/categories/${id}`, { headers: await authHeaders() });
    refresh();
  });
}
