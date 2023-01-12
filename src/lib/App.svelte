<script lang="ts">
    import { ScProvider } from '@polkadot/rpc-provider/substrate-connect';
    import { ApiPromise } from '@polkadot/api';
    import jsonCustomSpec from '../config/customSpecRaw.json';
    import * as Sc from '@substrate/connect';

    async function main() {
        // Create the provider for the custom chain
        const customSpec = JSON.stringify(jsonCustomSpec);
        const provider = new ScProvider(Sc, customSpec);
        // Stablish the connection (and catch possible errors)
        await provider.connect()
        // Create the PolkadotJS api instance
        const api = await ApiPromise.create({ provider });
        await api.rpc.chain.subscribeNewHeads((lastHeader) => {
        console.log(lastHeader.hash);
        });
        await api.disconnect();
    }

    main().catch(console.error);
</script>