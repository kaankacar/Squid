import { StellarSquidAgent } from './agent';
import * as fs from 'fs';
import * as path from 'path';

async function main() {
    const tempStateDir = path.join(__dirname, 'temp-state-verify');
    if (!fs.existsSync(tempStateDir)) {
        fs.mkdirSync(tempStateDir, { recursive: true });
    }

    try {
        console.log('--- Initializing Agent ---');
        const agent = new StellarSquidAgent({}, tempStateDir);

        console.log('\n--- Calling generateKeypair() ---');
        await agent.generateKeypair();

        console.log('\n--- Calling debug() ---');
        agent.debug();

    } finally {
        if (fs.existsSync(tempStateDir)) {
            fs.rmSync(tempStateDir, { recursive: true, force: true });
        }
    }
}

main().catch(err => {
    console.error(err);
    process.exit(1);
});
