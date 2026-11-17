import { FerroExports } from "./ferroExports";

interface RouteEntry {
    session: string;
    position: number;
    route: string;
}

interface RouteHistory {
    session: string;
    position: number;
    onPopState: (event: PopStateEvent) => void;
}

function routeEntry(state: { ferrouiNavigation?: RouteEntry } | null, session: string): RouteEntry | undefined {
    const entry = state?.ferrouiNavigation;
    if (entry?.session === session) {
        return entry;
    }
}

export class NavigationHelper {
    private static backHandlerInstalled = false;
    private static routeHistory?: RouteHistory;

    // Differs from the original, which is given the callback as a function: the export of the
    // framework is called instead. Also differs in that the "popstate" caused by its own
    // history.forward() (going back to the entry it pushed after a handled request) is not taken
    // for another back request: the original reports every back navigation twice.
    public static addBackHandler() {
        if (this.backHandlerInstalled || this.routeHistory !== undefined) { return; }
        history.pushState(null, "", window.location.href);
        this.backHandlerInstalled = true;
        let returningForward = false;
        window.addEventListener("popstate", () => {
            if (returningForward) {
                returningForward = false;
                return;
            }

            const handled: boolean = FerroExports.NavigationHelper?.OnBackRequested() ?? false;

            if (!handled) {
                history.back();
            } else {
                returningForward = true;
                history.forward();
            }
        });
    }

    public static startHistory(route: string): void {
        if (this.routeHistory !== undefined || this.backHandlerInstalled) {
            throw new Error("FerroUI browser navigation is already installed");
        }

        const session = crypto.getRandomValues(new Uint32Array(4)).join("-");
        const routes: RouteHistory = {
            session,
            position: 0,
            onPopState: (event) => {
                const entry = routeEntry(event.state, session);
                if (!entry || entry.position === routes.position) { return; }
                const delta = entry.position - routes.position;
                routes.position = entry.position;
                FerroExports.NavigationHelper?.OnHistoryTraversed?.(entry.route, delta);
            }
        };
        history.replaceState({ ferrouiNavigation: { session, position: 0, route } }, "");
        window.addEventListener("popstate", routes.onPopState);
        this.routeHistory = routes;
    }

    public static pushHistory(route: string): void {
        const routes = this.routeHistory;
        const current = routes && routeEntry(history.state, routes.session);
        if (!current) { throw new Error("The current browser entry is outside FerroUI navigation history"); }

        const position = current.position + 1;
        history.pushState({ ferrouiNavigation: { session: current.session, position, route } }, "");
        routes.position = position;
    }

    public static stopHistory(): void {
        const routes = this.routeHistory;
        if (!routes) { return; }
        window.removeEventListener("popstate", routes.onPopState);
        this.routeHistory = undefined;
    }

    public static openUri(uri?: string, target?: string) {
        return !!window.open(uri, target);
    }
}
