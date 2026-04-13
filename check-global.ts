import { Connection, PublicKey } from '@solana/web3.js';

const PROGRAM_ID = new PublicKey('31CyhhHdzLaZg2L4mkwRn9ZApzhzd4LyfjxiEoght9Fw');
const RPC = 'https://api.devnet.solana.com';

async function checkGlobal() {
  const [global] = PublicKey.findProgramAddressSync([Buffer.from('global')], PROGRAM_ID);
  console.log('Global PDA:', global.toBase58());
  
  const connection = new Connection(RPC, 'confirmed');
  const account = await connection.getAccountInfo(global);
  
  if (account) {
    console.log('✅ Global account exists');
  } else {
    console.log('❌ Global account NOT initialized - need to call initialize()');
  }
}

checkGlobal().catch(console.error);
