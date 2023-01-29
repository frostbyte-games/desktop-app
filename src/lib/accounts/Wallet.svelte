<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { activeAccount } from "./accounts";

  let account: string;
  activeAccount.subscribe(async (value: string) => {
    await setActiveAccount(value);
    account = value;
  });

  type Wallet = {
    address: string;
    balance: string;
  };

  let wallet: Promise<Wallet>;

  async function getBalance(account: string): Promise<Wallet> {
    return await invoke("balance", { account });
  }

  async function setActiveAccount(account: string): Promise<any> {
    return await invoke("set_active_account", { accountName: account });
  }

  $: wallet = getBalance(account);
</script>

<div class="col">
  {#await wallet then wallet}
    <p>{account} ({wallet.address}) have {wallet.balance} snowflakes</p>{/await}
</div>

<style>
  p {
    color: lightblue;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
