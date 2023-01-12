<script lang="ts">
    import { ApiPromise, WsProvider } from '@polkadot/api';

    async function bootstrapWsProvider(): Promise<ApiPromise> {
        const wsProvider = new WsProvider('ws://127.0.0.1:9944');
        return await ApiPromise.create({ provider: wsProvider });
    }

    async function main() {
        const api = await bootstrapWsProvider().catch((e) => {
            console.error(e);
            process.exit(1);
        });

        const [chain, nodeName, nodeVersion] = await Promise.all([
            api.rpc.system.chain(),
            api.rpc.system.name(),
            api.rpc.system.version(),
        ]);
        console.log(`You are connected to chain ${chain} using ${nodeName} v${nodeVersion}`);

        const lastHeader = await api.rpc.chain.getHeader();

        console.log(`The last block has hash ${lastHeader.hash}`);

        // The actual address that we will use
        const ADDR = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

        // Retrieve the last timestamp
        const now = await api.query.timestamp.now();

        // Retrieve the account balance & nonce via the system module
        const { nonce, data: balance } = await api.query.system.account(ADDR);

        console.log(`${now}: balance of ${balance.free} and a nonce of ${nonce}`);
    }

    main().catch(console.error);
</script>