<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";

  let name = "";
  let password = "";

  async function createAccount() {
    // add the account, encrypt the stored JSON with an account-specific password
    // const { pair, json } = keyring.addUri(mnemonic, password, { name });
  }

  let balanceMsg = "";

  async function balance() {
    balanceMsg = await invoke("balance");
  }

  balance();
</script>

<div class="col">
  <input id="name-input" placeholder="Enter a name..." bind:value={name} />
  <input
    id="password-input"
    placeholder="Enter a password..."
    bind:value={password}
  />
  <button on:click={createAccount}>Create Account</button>
  <ul>
    <!-- {#each accounts as account}
      <li>{account.address}</li>
    {/each} -->
  </ul>
  <p>{balanceMsg}</p>
</div>
