import { Button, Field, Sheet, StatusDot } from "@vpet/ds";

export const SyncSettings = () => (
  <Sheet
    title="Sync"
    hint="Share one pet between your phone and this browser."
    onClose={() => {}}
    actions={
      <>
        <Button>Save</Button>
        <Button variant="secondary">Sync now</Button>
        <Button variant="danger">Sign out</Button>
      </>
    }
  >
    <Field label="Server" defaultValue="https://vpet.example.com" />
    <Field label="Pet code" defaultValue="lalafu-7f3k" help="Enter the same code on every device." />
    <StatusDot level="ok">Synced 2 minutes ago</StatusDot>
  </Sheet>
);
export const Confirm = () => (
  <Sheet
    title="Start over?"
    hint="Your pet will be laid to rest and a new egg will appear."
    actions={
      <>
        <Button variant="danger">Start over</Button>
        <Button variant="ghost">Keep playing</Button>
      </>
    }
  >
    <p style={{ margin: 0 }}>Hold A and C on the device to do this without the sheet.</p>
  </Sheet>
);
