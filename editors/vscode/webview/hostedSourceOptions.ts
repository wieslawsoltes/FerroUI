export function sourceOptions(files: readonly { address: string; uri: string }[]): HTMLOptionElement[] {
    const labels = files.map(file => decodeURI(file.uri).split('/').pop() || file.uri);
    return files.map((file, index) => {
        const label = labels[index];
        const unique = labels.indexOf(label) === labels.lastIndexOf(label);
        const full = files.some(other => other !== file && other.uri === file.uri) ? file.address : file.uri;
        const option = new Option(unique ? label : decodeURI(full), file.address);
        option.title = decodeURI(full);
        return option;
    });
}
