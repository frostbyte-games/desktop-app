<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";

  let pubKey = "";
  let mnemonic = "";
  let name = "";
  let password = "";

  async function createAccount() {
    [pubKey, mnemonic] = await invoke("create_account", { name, password });
  }
</script>

<div class="col">
  <p>{mnemonic}</p>
  <p>{pubKey}</p>

  <input type="text" bind:value={name} />
  <input type="password" bind:value={password} />
  <button on:click={createAccount}>Create Account</button>
</div>

<style>
  p {
    color: coral;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
