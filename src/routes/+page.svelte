<script>
  import Accounts from "$lib/accounts/Accounts.svelte";
  import { invoke } from "@tauri-apps/api/tauri";

  let passwordEntered = false;
  let enteredPassword = "";

  async function unlock() {
    await invoke("unlock", { masterPassword: enteredPassword });

    passwordEntered = true;
  }
</script>

<h1>Welcome to Frostbyte!</h1>

<div>
  {#if !passwordEntered}
    <div>
      <form on:submit|preventDefault={unlock}>
        <label for="password">Enter Master Password:</label>
        <input type="password" id="password" bind:value={enteredPassword} />
        <button on:click={unlock} type="submit">Submit</button>
      </form>
    </div>
  {:else}
    <Accounts />
  {/if}
</div>
