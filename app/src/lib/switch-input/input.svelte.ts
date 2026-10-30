import { SwitchInput } from "./vendor"

/** Shared native input client so components don't overwrite one another's streams. */
export const switchInput = new SwitchInput()
