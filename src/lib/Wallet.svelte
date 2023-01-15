<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";

  let balance = "";

  async function getBalance() {
    balance = await invoke("balance");
  }

  getBalance();
</script>

<div class="col">
  <p>You have {balance} snowflakes!</p>
</div>

<style>
  p {
    color: lightblue;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
