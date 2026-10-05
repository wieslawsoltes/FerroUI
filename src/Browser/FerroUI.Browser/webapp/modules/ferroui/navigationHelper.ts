import { FerroExports } from "./ferroExports";

export class NavigationHelper {
    // Differs from the original, which is given the callback as a function: the export of the
    // framework is called instead. Also differs in that the "popstate" caused by its own
    // history.forward() (going back to the entry it pushed after a handled request) is not taken
    // for another back request: the original reports every back navigation twice.
    public static addBackHandler() {
        history.pushState(null, "", window.location.href);
        let returningForward = false;
        window.onpopstate = () => {
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
        };
    }

    public static openUri(uri?: string, target?: string) {
        return !!window.open(uri, target);
    }
}
