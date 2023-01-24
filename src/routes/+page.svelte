<script>
  import Accounts from "$lib/Accounts.svelte";

  let passwordEntered = false;
  let enteredPassword = "";

  function verifyPassword() {
    // check enteredPassword against stored master password
    if (enteredPassword === "masterpassword") {
      passwordEntered = true;
    } else {
      alert("Incorrect password. Please try again.");
    }
  }
</script>

<h1>Welcome to Frostbyte!</h1>

<div>
  {#if !passwordEntered}
    <div>
      <label for="password">Enter Master Password:</label>
      <input type="password" id="password" bind:value={enteredPassword} />
      <button on:click={verifyPassword}>Submit</button>
    </div>
  {:else}
    <Accounts />
  {/if}
</div>
