<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { writable } from "svelte/store";

  let pubKey = "";
  let mnemonic = "";
  let name = "";
  let masterPassword = "asdf";
  const loading = writable(false);
  let getAccountsResults: any = [];

  async function createAccount() {
    loading.set(true);
    [pubKey, mnemonic] = await invoke("create_account", {
      name,
      masterPassword
    });
    loading.set(false);
  }

  async function getAccounts() {
    loading.set(true);
    await invoke("get_accounts", { masterPassword })
      .then((result) => {
        getAccountsResults = (result as any).toString();
      })
      .catch((err) => {
        getAccountsResults = "Error: " + err.toString();
      });
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
    <p>{getAccountsResults}</p>
    <p>{mnemonic}</p>
    <p>{pubKey}</p>
  {/if}

  <input type="text" bind:value={name} />
  <input type="password" bind:value={masterPassword} />
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
