import { FerroExports } from "./ferroExports";

export class NavigationHelper {
    // Differs from the original, which is given the callback as a function: the export of the
    // framework is called instead.
    public static addBackHandler() {
        history.pushState(null, "", window.location.href);
        window.onpopstate = () => {
            const handled: boolean = FerroExports.NavigationHelper?.OnBackRequested() ?? false;

            if (!handled) {
                history.back();
            } else {
                history.forward();
            }
        };
    }

    public static openUri(uri?: string, target?: string) {
        return !!window.open(uri, target);
    }
}
