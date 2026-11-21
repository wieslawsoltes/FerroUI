import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { Range } from './protocol';
import type { RustStyleEdit, StyleCatalog, StyleCreation, StyleDefinition } from './stylesProtocol';

export interface HostedStyleFile { address: string; uri: string }
export interface HostedStyleTarget { address: string; node: number }
export interface HostedStyleFiles { documents: HostedStyleFile[] }
export interface HostedStyleCatalog { uri: string; catalog: StyleCatalog }
export interface HostedStyleDefinition { uri: string; definition: StyleDefinition<string> }
export interface HostedStyleResources { resources: { key: string; type_name: string }[] }

export type HostedStyleEdit = StyleCreation
    | { action: 'selector'; value: string }
    | { action: 'base'; key: string | null }
    | { action: 'setter'; property: string; value: string | null }
    | { action: 'resource'; property: string; key: string; dynamic: boolean }
    | { action: 'remove' }
    | { action: 'rust'; edit: RustStyleEdit };

export type HostedStyleRead = { action: 'files' } | { action: 'catalog'; address: string }
    | { action: 'read'; target: HostedStyleTarget };
export type NativeStyleRequest = HostedStyleRead
    | { action: 'resources'; target: HostedStyleTarget; property: string }
    | { action: 'plan'; target: HostedStyleTarget; edit: HostedStyleEdit; source: string }
    | HostedSourceRequest;
export type HostedSourceRequest = { action: 'source'; address: string } | { action: 'locate'; address: string; range: Range; source: string };
export type HostedStyleCommand = HostedStyleRead
    | { action: 'edit'; target: HostedStyleTarget; edit: HostedStyleEdit }
    | { action: 'locate'; address: string; range: Range }
    | { action: 'create'; address: string }
    | { action: 'issues'; address: string }
    | { action: 'addSetter'; target: HostedStyleTarget }
    | { action: 'addNested'; target: HostedStyleTarget }
    | { action: 'chooseResource'; target: HostedStyleTarget; property: string };
export type HostedStyleResult = HostedStyleFiles | HostedStyleCatalog | HostedStyleDefinition;
export type FromHostedStyles = { type: 'styles'; request: number; frame: HostedAnimationFrame; command: HostedStyleCommand };
export type ToHostedStyles = { type: 'stylesReply'; request: number; frame: HostedAnimationFrame; result?: HostedStyleResult; error?: string }
    | { type: 'openStyles'; revision: number; target?: HostedStyleTarget };
