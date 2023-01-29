<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { activeAccount } from "./accounts";
  import Wallet from "./Wallet.svelte";

  type Account = {
    password: string;
    address: string;
    mnemonic: string;
  };

  let account: Account;
  let name = "";

  async function createAccount() {
    account = await invoke("create_account", {
      name
    });
    accounts = getAccounts();
    activeAccount.set(name);
  }

  async function getAccounts(): Promise<string[]> {
    return await invoke("get_accounts");
  }

  let accounts = getAccounts().then((result) => {
    activeAccount.set(result[0]);
    return result;
  });
</script>

<div class="col">
  <input type="text" bind:value={name} placeholder="Name your account" />
  <button id="create-account" on:click={createAccount}>Create Account</button>
  {#await accounts}
    <div class="loading-spinner-wrapper">
      <div class="loading-spinner" />
    </div>
  {:then accounts}
    {#if accounts.length > 0}
      <select bind:value={$activeAccount}>
        {#each accounts as account}
          <option value={account}>{account}</option>
        {/each}
      </select>
      {#if account}
        <h2>New account created</h2>
        <p>
          Make sure to save your mnemonic phrase and your password to recover
          your account or import it on another device.
        </p>
        <h3>Address</h3>
        <p>{account.address}</p>
        <h3>Password</h3>
        <p>{account.password}</p>
        <h3>Mnemonic</h3>
        <p>{account.mnemonic}</p>
      {/if}
      <Wallet />
    {:else}
      <p>Create your first account!</p>
    {/if}
  {:catch error}
    <p style="color: red">getAccounts error: {error}</p>
  {/await}

  <!-- {#if $loading}{:else}
    <input type="text" bind:value={name} />
    <button on:click={createAccount}>Create Account</button> -->

  <!-- {/if} -->
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
  /* p {
    color: coral;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  } */
</style>
