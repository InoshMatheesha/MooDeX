// router.ts - Swap the main view; each view disposes its listeners and grid on exit.
export type Route = "dashboard" | "library" | "pc" | "discover" | "settings";

type Mount = (route: Route) => void;
let mountFn: Mount = () => {};

export function setMount(fn: Mount): void {
  mountFn = fn;
}

export function navigate(route: Route): void {
  mountFn(route);
}
