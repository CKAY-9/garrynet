import { Suspense } from "react";
import Link from "next/link";
import { forbidden } from "next/navigation";
import { requirePermission } from "@/lib/auth";
import { getForumHome } from "@/lib/forum";
import { hasPermission, Permission } from "@/lib/permissions";
import styles from "../admin.module.scss";
import { CreateCategoryForm, CreateSectionForm, DeleteButton } from "./ForumAdmin";
import list from "./forum.module.scss";

export default function ForumAdminPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <ForumAdmin />
    </Suspense>
  );
}

async function ForumAdmin() {
  const user = await requirePermission(Permission.AdminAccess);
  const sections = hasPermission(user.permissions, Permission.SectionManagement);
  const categories = hasPermission(user.permissions, Permission.CategoryManagement);
  if (!sections && !categories) forbidden();

  const forum = await getForumHome();

  return (
    <>
      <section className={styles.section} aria-labelledby="structure-heading">
        <h2 id="structure-heading" className={styles.sectionTitle}>
          Sections and categories
        </h2>
        {forum.sections.length === 0 ? (
          <p className={styles.placeholder}>No sections yet. Create one below.</p>
        ) : (
          <ul className={list.sections}>
            {forum.sections.map((section) => (
              <li key={section.id} className={list.section} style={{ borderLeftColor: section.accent }}>
                <div className={list.item}>
                  <span className={list.itemText}>
                    <strong>{section.name}</strong>
                    <span className={list.meta}>
                      position {section.position}
                      {section.description && ` · ${section.description}`}
                    </span>
                  </span>
                  {sections && <DeleteButton kind="section" id={section.id} name={section.name} />}
                </div>
                <ul className={list.categories}>
                  {section.categories.map((c) => (
                    <li key={c.id} className={list.item}>
                      <span className={list.itemText}>
                        <Link href={`/forum/${c.id}`}>{c.name}</Link>
                        <span className={list.meta}>
                          position {c.position} · {c.thread_count} threads · {c.post_count} posts
                          {c.locked && " · locked"}
                        </span>
                      </span>
                      {categories && <DeleteButton kind="category" id={c.id} name={c.name} />}
                    </li>
                  ))}
                  {section.categories.length === 0 && <li className={list.meta}>No categories.</li>}
                </ul>
              </li>
            ))}
          </ul>
        )}
      </section>

      {sections && (
        <section className={styles.section} aria-labelledby="new-section-heading">
          <h2 id="new-section-heading" className={styles.sectionTitle}>
            New section
          </h2>
          <CreateSectionForm />
        </section>
      )}

      {categories && (
        <section className={styles.section} aria-labelledby="new-category-heading">
          <h2 id="new-category-heading" className={styles.sectionTitle}>
            New category
          </h2>
          <CreateCategoryForm sections={forum.sections.map(({ id, name }) => ({ id, name }))} />
        </section>
      )}
    </>
  );
}
