<script>
  import Accounts from "$lib/accounts/Accounts.svelte";
  import { invoke } from "@tauri-apps/api/tauri";

  let passwordEntered = false;
  let enteredPassword = "";
  let errorMessage = "";

  const invalidPasswordError = "InvalidPassword";

  async function unlock() {
    if (enteredPassword.length < 32) {
      errorMessage = "Password must be at least 32 characters long";
      return;
    }

    await invoke("unlock", { masterPassword: enteredPassword })
      .then(() => {
        passwordEntered = true;
      })
      .catch((err) => {
        if (err === invalidPasswordError) {
          errorMessage = "Invalid Password";
        }
      });
  }
</script>

<h1>Welcome to Frostbyte!</h1>

<div>
  {#if !passwordEntered}
    <div>
      <form on:submit|preventDefault={unlock}>
        <label for="password">Enter Master Password:</label>
        <input type="password" id="password" bind:value={enteredPassword} />
        <button on:click={unlock} type="submit">Unlock</button>
      </form>
      <p style="color: red">{errorMessage}</p>
    </div>
  {:else}
    <Accounts />
  {/if}
</div>
