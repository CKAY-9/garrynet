import { Suspense } from "react";
import { notFound, unauthorized } from "next/navigation";
import { TicketView } from "@/components/Support";
import { getSupportViewer, getTicket } from "@/lib/support";

export default function TicketPage({ params }: PageProps<"/support/ticket/[ticket_id]">) {
  return (
    // Params, the ticket and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ ticket_id }) => (
        <Ticket id={ticket_id} />
      ))}
    </Suspense>
  );
}

/** /support/ticket/{id}: for the ticket's owner and the Support Team. */
async function Ticket({ id }: { id: string }) {
  const viewer = await getSupportViewer();
  if (!viewer.userId) unauthorized();
  // The backend answers 404 for other people's tickets too.
  const page = await getTicket(id);
  if (!page) notFound();
  return <TicketView page={page} viewer={viewer} />;
}
