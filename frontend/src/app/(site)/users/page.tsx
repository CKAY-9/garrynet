import { redirect } from "next/navigation";

/** `/users` has no page of its own; user browsing lives on the search page. */
export default function UsersPage() {
  redirect("/users/search");
}
