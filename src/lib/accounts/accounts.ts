import { writable } from "svelte/store";

export const activeAccount = writable("");

export type Keystore = {
  public_key: any;
  signature: any;
  message: any;
};
