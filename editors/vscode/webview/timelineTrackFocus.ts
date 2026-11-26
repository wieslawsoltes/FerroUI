function canFocus(element: HTMLElement | null): element is HTMLElement {
    return Boolean(element?.isConnected && !element.closest('[hidden], :disabled'));
}

/** A track can remain readable while it waits for an accepted source revision. */
export function setTrackButtonDisabled(button: HTMLButtonElement, disabled: boolean): void {
    if (disabled) button.setAttribute('aria-disabled', 'true');
    else button.removeAttribute('aria-disabled');
    button.tabIndex = disabled ? -1 : 0;
}

export function preserveTrackFocus(
    tracks: HTMLElement,
    fallbacks: readonly HTMLElement[],
    update: () => boolean | void,
): void {
    const active = document.hasFocus() ? document.activeElement : null;
    const focused = active instanceof HTMLElement && tracks.contains(active) ? active : undefined;
    const group = focused?.closest('.timeline-channel')?.querySelector('.timeline-channel-toggle');
    const sameProgram = update() !== false;
    if (!focused || (document.activeElement !== focused && document.activeElement !== document.body)) return;
    if (sameProgram && canFocus(focused)) return;

    const replacement = sameProgram && focused.id ? document.getElementById(focused.id) : null;
    const target = sameProgram && group?.id ? document.getElementById(group.id) : null;
    [replacement, target, ...fallbacks].find(canFocus)?.focus({ preventScroll: true });
}
