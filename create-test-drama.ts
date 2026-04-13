import { Connection, Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import { Program, AnchorProvider, Wallet, BN } from '@coral-xyz/anchor';
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from '@solana/spl-token';
import { readFileSync } from 'fs';
import { homedir } from 'os';

const PROGRAM_ID = new PublicKey('31CyhhHdzLaZg2L4mkwRn9ZApzhzd4LyfjxiEoght9Fw');
const MPL_TOKEN_METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');
const RPC = 'https://api.devnet.solana.com';

const [EVENT_AUTHORITY] = PublicKey.findProgramAddressSync([Buffer.from('__event_authority')], PROGRAM_ID);

async function createTestDrama() {
  console.log('🎬 Creating test drama...\n');

  // Load wallet
  const walletPath = `${homedir()}/.config/solana/id.json`;
  const keypair = Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(readFileSync(walletPath, 'utf-8')))
  );
  console.log('✅ Wallet:', keypair.publicKey.toBase58());

  const connection = new Connection(RPC, 'confirmed');
  const wallet = new Wallet(keypair);
  const provider = new AnchorProvider(connection, wallet, {});

  // Load IDL
  const idlPath = '/tmp/reel-fun-orig/src/idl/reelfun.json';
  const idl = JSON.parse(readFileSync(idlPath, 'utf-8'));
  const program = new Program(idl, PROGRAM_ID, provider);

  // Generate mint
  const mint = Keypair.generate();
  console.log('🪙 Mint:', mint.publicKey.toBase58());

  const [bondingCurve] = PublicKey.findProgramAddressSync(
    [Buffer.from('bonding-curve'), mint.publicKey.toBuffer()],
    PROGRAM_ID
  );

  const [global] = PublicKey.findProgramAddressSync([Buffer.from('global')], PROGRAM_ID);

  const bondingCurveTokenAccount = await getAssociatedTokenAddress(
    mint.publicKey,
    bondingCurve,
    true
  );

  const [metadata] = PublicKey.findProgramAddressSync(
    [Buffer.from('metadata'), MPL_TOKEN_METADATA_PROGRAM_ID.toBuffer(), mint.publicKey.toBuffer()],
    MPL_TOKEN_METADATA_PROGRAM_ID
  );

  const [whitelist] = PublicKey.findProgramAddressSync(
    [Buffer.from('whitelist'), keypair.publicKey.toBuffer()],
    PROGRAM_ID
  );

  console.log('📝 Creating bonding curve...');
  
  const tx = await program.methods
    .createBondingCurve({ 
      name: 'Test Drama Token', 
      symbol: 'TEST', 
      uri: '', 
      startTime: null 
    })
    .accounts({
      mint: mint.publicKey,
      creator: keypair.publicKey,
      bondingCurve,
      bondingCurveTokenAccount,
      global,
      whitelist,
      metadata,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
      rent: new PublicKey('SysvarRent111111111111111111111111111111111'),
    })
    .remainingAccounts([
      { pubkey: EVENT_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: PROGRAM_ID, isSigner: false, isWritable: false },
    ])
    .signers([mint])
    .rpc();

  console.log('✅ Transaction:', tx);
  console.log('✅ Mint:', mint.publicKey.toBase58());
  console.log('✅ Bonding Curve:', bondingCurve.toBase58());

  // Create drama in backend
  console.log('\n📝 Creating drama in backend...');
  const response = await fetch('https://reel.fun/api/drama/create', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      title: 'Test Drama - Solana Devnet',
      description: 'A test drama created on Solana Devnet for testing the bonding curve mechanism.',
      style: 'THRILLER',
      protagonistIdentity: 'A blockchain developer',
      coreRelationship: 'Developer vs Bug',
      wallet: keypair.publicKey.toBase58(),
      tokenMint: mint.publicKey.toBase58(),
      bondingCurve: tx,
    }),
  });

  const data = await response.json();
  if (data.success) {
    console.log('✅ Drama created:', data.drama.id);
    console.log('🎉 Done! Check https://reel.fun');
  } else {
    console.log('❌ Failed:', data.error);
  }
}

createTestDrama().catch(console.error);
