import { spawn } from "node:child_process";

export async function startXvfb() {
    // Linux clients can use the abstract local socket. The filesystem socket is unnecessary
    // and prevents display allocation on WSL when its X11 socket directory is managed by WSLg.
    const process = spawn("Xvfb", ["-displayfd", "3", "-screen", "0", "1200x850x24", "-noreset",
        "-nolisten", "tcp", "-nolisten", "unix"], {
        stdio: ["ignore", "ignore", "inherit", "pipe"],
    });
    try {
        const display = await new Promise((resolve, reject) => {
            const timeout = setTimeout(() => reject(new Error("Xvfb did not allocate a display")), 8000);
            process.once("error", error => { clearTimeout(timeout); reject(error); });
            process.once("exit", code => { clearTimeout(timeout); reject(new Error(`Xvfb exited with ${code}`)); });
            let text = "";
            process.stdio[3].setEncoding("utf8").on("data", part => {
                text += part;
                if (!text.includes("\n")) return;
                clearTimeout(timeout);
                const number = text.trim();
                if (/^\d+$/.test(number)) resolve(`:${number}`);
                else reject(new Error("Invalid Xvfb display number"));
            });
        });
        return { process, display };
    } catch (error) {
        process.kill();
        throw error;
    }
}
