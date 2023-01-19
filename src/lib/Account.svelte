<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { writable } from "svelte/store";
  import type { Keystore } from "./accounts";

  let pubKey = "";
  let mnemonic = "";
  let name = "";
  let password = "";
  const loading = writable(false);
  let account: Keystore;

  async function createAccount() {
    loading.set(true);
    [password, pubKey, mnemonic] = await invoke("create_account", {
      name
    });
    loading.set(false);
  }

  async function getAccounts() {
    loading.set(true);
    account = await invoke("get_accounts");
    loading.set(false);
  }

  getAccounts();
</script>

<div class="col">
  {#if $loading}
    <div class="loading-spinner-wrapper">
      <div class="loading-spinner" />
    </div>
  {:else}
    <ul>
      <li>Address: {account.public_key}</li>
      <li>Signature: {account.signature}</li>
      <li>Message: {account.message}</li>
    </ul>
    <p>{password}</p>
    <p>{mnemonic}</p>
    <p>{pubKey}</p>
  {/if}

  <input type="text" bind:value={name} />
  <button on:click={createAccount}>Create Account</button>
</div>

<style>
  .loading-spinner-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .loading-spinner {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    border: 6px solid #f3f3f3;
    border-top-color: coral;
    animation: spin 1s linear infinite;
    margin: 0 auto;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
  p {
    color: coral;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
