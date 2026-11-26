export type RoomRole = 'owner' | 'editor' | 'viewer';
export type GuestRole = Exclude<RoomRole, 'owner'>;

export interface RoomMember {
    actor: string;
    display_name: string;
    role: RoomRole;
    revoked: boolean;
}

export interface RoomStatus {
    title: string;
    actor: string;
    role: RoomRole;
    phase: 'connected' | 'synchronizing' | 'reconnecting' | 'review' | 'conflict' | 'blocked';
    message: string | null;
    members: RoomMember[];
    online: string[];
    conflicts: { reason: string }[];
    pending: number;
    revision: string | null;
}

export interface RoomReview {
    revision: string;
    source: string;
    rebase: boolean;
}

export type RoomStart =
    | { action: 'host'; title: string; source: string; display_name: string; public_url?: string; bind?: string }
    | { action: 'join'; invitation: string; display_name: string }
    | { action: 'resume' };

export type RoomCommand =
    | { action: 'open'; options: { directory: string; uri: string; start: RoomStart } }
    | { action: 'tick'; refresh: boolean }
    | { action: 'review' | 'recovery' }
    | { action: 'prepare_apply'; revision: string; before: string }
    | { action: 'publish'; revision: string; source: string }
    | { action: 'invite'; role: GuestRole }
    | { action: 'member'; actor: string; role: GuestRole; revoked: boolean }
    | { action: 'resolve' | 'checkpoint'; revision: string; local: string; resolved?: string };
