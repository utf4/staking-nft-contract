use solana_program::program::{invoke_signed, invoke};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    msg,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, Sysvar, rent::Rent},
    self,
};
use solana_program::borsh::try_from_slice_unchecked;
use borsh::{BorshDeserialize, BorshSerialize,BorshSchema};
use spl_token;
use spl_associated_token_account;
use spl_token_metadata;


// Declare and export the program's entrypoint
entrypoint!(process_instruction);

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
enum StakeInstruction{
    GenerateVault{
        #[allow(dead_code)]
        min_period:u64,
        #[allow(dead_code)]
        reward_period:u64,
    },
    Stake,
    Unstake,
    AddToWhitelist{
        #[allow(dead_code)]
        price:u64,
    },
    Withdraw{
        #[allow(dead_code)]
        amount:u64,
    },
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
struct StakeData{
    timestamp: u64,
    staker: Pubkey,
    active: bool,
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
struct ContractData{
    min_period: u64,
    reward_period: u64,
}


#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
struct RateData{
    price: u64,
}

// Program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let instruction: StakeInstruction = try_from_slice_unchecked(instruction_data).unwrap();
    let vault_word = "vault";
    let whitelist_word = "whitelist";

    let admin = "Ek6Vqf4cCq6zXAp9TwSqeAbQXm8Eo3Y8DV7abbJYntwv".parse::<Pubkey>().unwrap();
    let reward_mint = "Aoz9EBZPZ8oQHnuV8UY5bCV87xJ5DpwFcy84TrRWBCzp".parse::<Pubkey>().unwrap();

    match instruction{
        StakeInstruction::Withdraw{amount}=>{
            let payer = next_account_info(accounts_iter)?;
            let payer_reward_holder_info = next_account_info(accounts_iter)?;
            let vault_reward_holder_info = next_account_info(accounts_iter)?;
            let vault_info = next_account_info(accounts_iter)?;
            let reward_mint_info = next_account_info(accounts_iter)?;

            let system_program = next_account_info(accounts_iter)?;
            let token_info = next_account_info(accounts_iter)?;
            let rent_info = next_account_info(accounts_iter)?;
            let assoc_acccount_info = next_account_info(accounts_iter)?;

            if *payer.key!=admin||!payer.is_signer{
                //unauthorized access
                return Err(ProgramError::Custom(0x231));
            }

            let ( vault_address, vault_bump ) = Pubkey::find_program_address(&[&vault_word.as_bytes()], &program_id);
            let payer_reward_holder = spl_associated_token_account::get_associated_token_address(payer.key, &reward_mint);
            let vault_reward_holder = spl_associated_token_account::get_associated_token_address(vault_info.key, &reward_mint);

            if vault_address!=*vault_info.key{
                //wrong stake_info
                return Err(ProgramError::Custom(0x261));
            }

            if payer_reward_holder!=*payer_reward_holder_info.key{
                //wrong payer_reward_holder_info
                return Err(ProgramError::Custom(0x262));
            }

            if vault_reward_holder!=*vault_reward_holder_info.key{
                //wrong vault_reward_holder_info
                return Err(ProgramError::Custom(0x263));
            }

            if payer_reward_holder_info.owner != token_info.key{
                invoke(
                    &spl_associated_token_account::create_associated_token_account(
                        payer.key,
                        payer.key,
                        reward_mint_info.key,
                    ),
                    &[
                        payer.clone(), 
                        payer_reward_holder_info.clone(), 
                        payer.clone(),
                        reward_mint_info.clone(),
                        system_program.clone(),
                        token_info.clone(),
                        rent_info.clone(),
                        assoc_acccount_info.clone(),
                    ],
                    
                )?;
            }

            invoke_signed(
                &spl_token::instruction::transfer(
                    token_info.key,
                    vault_reward_holder_info.key,
                    payer_reward_holder_info.key,
                    vault_info.key,
                    &[],
                    amount,
                )?,
                &[
                    vault_reward_holder_info.clone(),
                    payer_reward_holder_info.clone(),
                    vault_info.clone(), 
                    token_info.clone()
                ],
                &[&[&vault_word.as_bytes(), &[vault_bump]]],
            )?;
        },
        StakeInstruction::AddToWhitelist{price}=>{
            let payer = next_account_info(accounts_iter)?;
            let candy_machine_info = next_account_info(accounts_iter)?;
            let whitelist_info = next_account_info(accounts_iter)?;
            let sys_info = next_account_info(accounts_iter)?;
            let rent_info = next_account_info(accounts_iter)?;

            let rent = &Rent::from_account_info(rent_info)?;

            if *payer.key!=admin||!payer.is_signer{
                //unauthorized access
                return Err(ProgramError::Custom(0x31));
            }

            let (data_address,data_address_bump) = Pubkey::find_program_address(&[whitelist_word.as_bytes(), &candy_machine_info.key.to_bytes()], &program_id);
            if *whitelist_info.key!=data_address{
                //wrong whitelist_info
                return Err(ProgramError::Custom(0x32));
            }

            // if candy_machine_info.owner.to_string() != "cndyAnrLdpjq1Ssp1z8xxDsB8dxe7u4HL5Nxi2K5WXZ" {
            //     // msg!("invalid candy machine");
            //     return Err(ProgramError::Custom(0x33));
            // }

            let size = 8;
            if whitelist_info.owner!=program_id{
                let required_lamports = rent
                .minimum_balance(size as usize)
                .max(1)
                .saturating_sub(whitelist_info.lamports());
                invoke(
                    &system_instruction::transfer(payer.key, &data_address, required_lamports),
                    &[
                        payer.clone(),
                        whitelist_info.clone(),
                        sys_info.clone(),
                    ],
                )?;
                invoke_signed(
                    &system_instruction::allocate(&data_address, size),
                    &[
                        whitelist_info.clone(),
                        sys_info.clone(),
                    ],
                    &[&[whitelist_word.as_bytes(), &candy_machine_info.key.to_bytes(), &[data_address_bump]]],
                )?;

                invoke_signed(
                    &system_instruction::assign(&data_address, program_id),
                    &[
                        whitelist_info.clone(),
                        sys_info.clone(),
                    ],
                    &[&[whitelist_word.as_bytes(), &candy_machine_info.key.to_bytes(), &[data_address_bump]]],
                )?;
            }

            let rate_struct = RateData{
                price,
            };
            rate_struct.serialize(&mut &mut whitelist_info.data.borrow_mut()[..])?;
        },


        StakeInstruction::Unstake=>{
            let payer = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;
            let nft_info = next_account_info(accounts_iter)?;
            let token_info = next_account_info(accounts_iter)?;
            let rent_info = next_account_info(accounts_iter)?;
            let assoc_acccount_info = next_account_info(accounts_iter)?;
            let stake_info = next_account_info(accounts_iter)?;
            let vault_info = next_account_info(accounts_iter)?;
            let payer_reward_holder_info = next_account_info(accounts_iter)?;
            let vault_reward_holder_info = next_account_info(accounts_iter)?;
            let payer_nft_holder_info = next_account_info(accounts_iter)?;
            let vault_nft_holder_info = next_account_info(accounts_iter)?;
            let metadata_info = next_account_info(accounts_iter)?;
            
            let whitelist_info = next_account_info(accounts_iter)?;
            let reward_mint_info = next_account_info(accounts_iter)?;

            let clock = Clock::get()?;

            let ( stake_address, _stake_bump ) = Pubkey::find_program_address(&[&nft_info.key.to_bytes()], &program_id);
            let ( vault_address, vault_bump ) = Pubkey::find_program_address(&[&vault_word.as_bytes()], &program_id);
            let payer_reward_holder = spl_associated_token_account::get_associated_token_address(payer.key, &reward_mint);
            let vault_reward_holder = spl_associated_token_account::get_associated_token_address(vault_info.key, &reward_mint);
            let payer_nft_holder = spl_associated_token_account::get_associated_token_address(payer.key, nft_info.key);
            let vault_nft_holder = spl_associated_token_account::get_associated_token_address(vault_info.key, nft_info.key);
            let (metadata_address,_) =Pubkey::find_program_address(&["metadata".as_bytes(), &spl_token_metadata::ID.to_bytes(), &nft_info.key.to_bytes()], &spl_token_metadata::ID);

            
            if *token_info.key!=spl_token::id(){
                //wrong token_info
                return Err(ProgramError::Custom(0x345));
            }

            if stake_address!=*stake_info.key{
                //wrong stake_info
                return Err(ProgramError::Custom(0x60));
            }

            if vault_address!=*vault_info.key{
                //wrong stake_info
                return Err(ProgramError::Custom(0x61));
            }

            if payer_reward_holder!=*payer_reward_holder_info.key{
                //wrong payer_reward_holder_info
                return Err(ProgramError::Custom(0x62));
            }

            if vault_reward_holder!=*vault_reward_holder_info.key{
                //wrong vault_reward_holder_info
                return Err(ProgramError::Custom(0x63));
            }

            if payer_nft_holder!=*payer_nft_holder_info.key{
                //wrong payer_nft_holder_info
                return Err(ProgramError::Custom(0x64));
            }

            if vault_nft_holder!=*vault_nft_holder_info.key{
                //wrong vault_nft_holder_info
                return Err(ProgramError::Custom(0x65));
            }

            if metadata_address!=*metadata_info.key{
                //wrong metadata_info
                return Err(ProgramError::Custom(0x66));
            }

            if reward_mint!=*reward_mint_info.key{
                //wrong reward_mint_info
                return Err(ProgramError::Custom(0x67));
            }

            let metadata = spl_token_metadata::state::Metadata::from_account_info(metadata_info)?;
            let creators = metadata.data.creators.unwrap();
            let cndy = creators.first().unwrap();
            let candy_machine = cndy.address;

            // if candy_machine != *candy_machine_info.key {
            //     //msg!("Wrong candy machine");
            //     return Err(ProgramError::Custom(0x104));
            // }

            let (wl_data_address,_wl_data_address_bump) = Pubkey::find_program_address(&[whitelist_word.as_bytes(), &candy_machine.to_bytes()], &program_id);

            if *whitelist_info.key != wl_data_address{
                // wrong whitelist_info
                return Err(ProgramError::Custom(0x910));
            }

            let wl_rate_data = if let Ok(data) = RateData::try_from_slice(&whitelist_info.data.borrow()){
                data.price
            } else {
                // can't deserialize rate data
                return Err(ProgramError::Custom(0x911));
            };

            let vault_data = if let Ok(data) = ContractData::try_from_slice(&vault_info.data.borrow()){
                data
            } else {
                // can't deserialize vault data
                return Err(ProgramError::Custom(0x912));
            };

            let mut stake_data = if let Ok(data) = StakeData::try_from_slice(&stake_info.data.borrow()){
                data
            } else {
                // can't deserialize stake data
                return Err(ProgramError::Custom(0x913));
            };

            if !cndy.verified{
                //msg!("address is not verified");
                return Err(ProgramError::Custom(0x106));
            }

            if !stake_data.active{
                //staking is inactive
                return Err(ProgramError::Custom(0x107));
            }

            if stake_data.staker!=*payer.key{
                //unauthorized access
                return Err(ProgramError::Custom(0x108));
            }

            if clock.unix_timestamp as u64-stake_data.timestamp < vault_data.min_period{
                //can't unstake because minimal period of staking is not reached yet
                return Err(ProgramError::Custom(0x109));
            }
            msg!("periods passed {:?}",(clock.unix_timestamp as u64-stake_data.timestamp)/vault_data.reward_period);
            let reward = (clock.unix_timestamp as u64-stake_data.timestamp)/vault_data.reward_period*wl_rate_data;

            if payer_reward_holder_info.owner != token_info.key{
                invoke(
                    &spl_associated_token_account::create_associated_token_account(
                        payer.key,
                        payer.key,
                        reward_mint_info.key,
                    ),
                    &[
                        payer.clone(), 
                        payer_reward_holder_info.clone(), 
                        payer.clone(),
                        reward_mint_info.clone(),
                        system_program.clone(),
                        token_info.clone(),
                        rent_info.clone(),
                        assoc_acccount_info.clone(),
                    ],
                    
                )?;
            }

            invoke_signed(
                &spl_token::instruction::transfer(
                    token_info.key,
                    vault_reward_holder_info.key,
                    payer_reward_holder_info.key,
                    vault_info.key,
                    &[],
                    reward,
                )?,
                &[
                    vault_reward_holder_info.clone(),
                    payer_reward_holder_info.clone(),
                    vault_info.clone(), 
                    token_info.clone()
                ],
                &[&[&vault_word.as_bytes(), &[vault_bump]]],
            )?;


            if payer_nft_holder_info.owner != token_info.key{
                invoke(
                    &spl_associated_token_account::create_associated_token_account(
                        payer.key,
                        payer.key,
                        nft_info.key,
                    ),
                    &[
                        payer.clone(), 
                        payer_nft_holder_info.clone(), 
                        payer.clone(),
                        nft_info.clone(),
                        system_program.clone(),
                        token_info.clone(),
                        rent_info.clone(),
                        assoc_acccount_info.clone(),
                    ],
                    
                )?;
            }

            invoke_signed(
                &spl_token::instruction::transfer(
                    token_info.key,
                    vault_nft_holder_info.key,
                    payer_nft_holder_info.key,
                    vault_info.key,
                    &[],
                    1,
                )?,
                &[
                    vault_nft_holder_info.clone(),
                    payer_nft_holder_info.clone(),
                    vault_info.clone(), 
                    token_info.clone()
                ],
                &[&[&vault_word.as_bytes(), &[vault_bump]]],
            )?;

            invoke_signed(
                &spl_token::instruction::close_account(
                    token_info.key,
                    vault_nft_holder_info.key,
                    payer.key,
                    vault_info.key,
                    &[],
                )?,
                &[
                    vault_nft_holder_info.clone(),
                    payer.clone(),
                    vault_info.clone(), 
                    token_info.clone()
                ],
                &[&[&vault_word.as_bytes(), &[vault_bump]]],
            )?;
            stake_data.active=false;
            stake_data.serialize(&mut &mut stake_info.data.borrow_mut()[..])?;
        },
        
        StakeInstruction::Stake=>{
            let payer = next_account_info(accounts_iter)?;
            let mint = next_account_info(accounts_iter)?;
            let metadata_account_info = next_account_info(accounts_iter)?;
            
            let vault_info = next_account_info(accounts_iter)?;
            let source = next_account_info(accounts_iter)?;
            let destination = next_account_info(accounts_iter)?;

            let token_program = next_account_info(accounts_iter)?;
            let sys_info = next_account_info(accounts_iter)?;
            let rent_info = next_account_info(accounts_iter)?;
            let token_assoc = next_account_info(accounts_iter)?;
            
            let stake_data_info = next_account_info(accounts_iter)?;
            let whitelist_info = next_account_info(accounts_iter)?;

            let clock = Clock::get()?;

            if *token_program.key!=spl_token::id(){
                //wrong token_info
                return Err(ProgramError::Custom(0x345));
            }

            let rent = &Rent::from_account_info(rent_info)?;
            let ( stake_data, stake_data_bump ) = Pubkey::find_program_address(&[&mint.key.to_bytes()], &program_id);

            if !payer.is_signer{
                //unauthorized access
                return Err(ProgramError::Custom(0x11));
            }

            if stake_data!=*stake_data_info.key{
                //msg!("invalid stake_data account!");
                return Err(ProgramError::Custom(0x10));
            }

            let size: u64 = 8+32+1;
            if stake_data_info.owner != program_id{
                let required_lamports = rent
                .minimum_balance(size as usize)
                .max(1)
                .saturating_sub(stake_data_info.lamports());
                invoke(
                    &system_instruction::transfer(payer.key, &stake_data, required_lamports),
                    &[
                        payer.clone(),
                        stake_data_info.clone(),
                        sys_info.clone(),
                    ],
                )?;
                invoke_signed(
                    &system_instruction::allocate(&stake_data, size),
                    &[
                        stake_data_info.clone(),
                        sys_info.clone(),
                    ],
                    &[&[&mint.key.to_bytes(), &[stake_data_bump]]],
                )?;

                invoke_signed(
                    &system_instruction::assign(&stake_data, program_id),
                    &[
                        stake_data_info.clone(),
                        sys_info.clone(),
                    ],
                    &[&[&mint.key.to_bytes(), &[stake_data_bump]]],
                )?;
            }

            let stake_struct = StakeData{
                timestamp: clock.unix_timestamp as u64,
                staker: *payer.key,
                active: true,
            };
            stake_struct.serialize(&mut &mut stake_data_info.data.borrow_mut()[..])?;

            if &Pubkey::find_program_address(&["metadata".as_bytes(), &spl_token_metadata::ID.to_bytes(), &mint.key.to_bytes()], &spl_token_metadata::ID).0 != metadata_account_info.key {
                //msg!("invalid metadata account!");
                return Err(ProgramError::Custom(0x03));
            }

            let metadata = spl_token_metadata::state::Metadata::from_account_info(metadata_account_info)?;
            let creators = metadata.data.creators.unwrap();
            let cndy = creators.first().unwrap();
            let candy_machine = cndy.address;


            // if candy_machine != *candy_machine_info.key {
            //     //msg!("Wrong candy machine");
            //     return Err(ProgramError::Custom(0x04));
            // }

            let (wl_data_address,_wl_data_address_bump) = Pubkey::find_program_address(&[whitelist_word.as_bytes(), &candy_machine.to_bytes()], &program_id);

            if *whitelist_info.key != wl_data_address{
                // wrong whitelist_info
                return Err(ProgramError::Custom(0x900));
            }

            if whitelist_info.owner != program_id{
                // candy machine is not whitelisted
                return Err(ProgramError::Custom(0x902));
            }

            let _wl_rate_data = if let Ok(data) = RateData::try_from_slice(&whitelist_info.data.borrow()){
                data.price
            } else {
                // can't deserialize rate data
                return Err(ProgramError::Custom(0x901));
            };


            // if candy_machine_info.owner.to_string() != "cndyAnrLdpjq1Ssp1z8xxDsB8dxe7u4HL5Nxi2K5WXZ" {
            //     // msg!("invalid candy machine");
            //     return Err(ProgramError::Custom(0x05));
            // }

            if !cndy.verified{
                //msg!("address is not verified");
                return Err(ProgramError::Custom(0x06));
            }

            let ( vault, _vault_bump ) = Pubkey::find_program_address(&[&vault_word.as_bytes()], &program_id);
            if vault != *vault_info.key{
                //msg!("Wrong vault");
                return Err(ProgramError::Custom(0x07));
            }

            if &spl_associated_token_account::get_associated_token_address(payer.key, mint.key) != source.key {
                // msg!("Wrong source");
                return Err(ProgramError::Custom(0x08));
            }

            if &spl_associated_token_account::get_associated_token_address(&vault, mint.key) != destination.key{
                //msg!("Wrong destination");
                return Err(ProgramError::Custom(0x09));
            }

            if destination.owner != token_program.key{
                invoke(
                    &spl_associated_token_account::create_associated_token_account(
                        payer.key,
                        vault_info.key,
                        mint.key,
                    ),
                    &[
                        payer.clone(), 
                        destination.clone(), 
                        vault_info.clone(),
                        mint.clone(),
                        sys_info.clone(),
                        token_program.clone(),
                        rent_info.clone(),
                        token_assoc.clone(),
                    ],
                )?;
            }
            invoke(
                &spl_token::instruction::transfer(
                    token_program.key,
                    source.key,
                    destination.key,
                    payer.key,
                    &[],
                    1,
                )?,
                &[
                    source.clone(),
                    destination.clone(),
                    payer.clone(), 
                    token_program.clone()
                ],
            )?;

        },

        StakeInstruction::GenerateVault{min_period,reward_period}=>{
            let payer = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;
            let pda = next_account_info(accounts_iter)?;
            let rent_info = next_account_info(accounts_iter)?;

            let rent = &Rent::from_account_info(rent_info)?;

            let (vault_pda, vault_bump_seed) =
                Pubkey::find_program_address(&[vault_word.as_bytes()], &program_id);
            
            if pda.key!=&vault_pda{
                //msg!("Wrong account generated by client");
                return Err(ProgramError::Custom(0x00));
            }

            if pda.owner!=program_id{
                let size = 16;
           
                let required_lamports = rent
                .minimum_balance(size as usize)
                .max(1)
                .saturating_sub(pda.lamports());

                invoke(
                    &system_instruction::transfer(payer.key, &vault_pda, required_lamports),
                    &[
                        payer.clone(),
                        pda.clone(),
                        system_program.clone(),
                    ],
                )?;

                invoke_signed(
                    &system_instruction::allocate(&vault_pda, size),
                    &[
                        pda.clone(),
                        system_program.clone(),
                    ],
                    &[&[vault_word.as_bytes(), &[vault_bump_seed]]],
                )?;

                invoke_signed(
                    &system_instruction::assign(&vault_pda, program_id),
                    &[
                        pda.clone(),
                        system_program.clone(),
                    ],
                    &[&[vault_word.as_bytes(), &[vault_bump_seed]]],
                )?;
            }

            if *payer.key!=admin||!payer.is_signer{
                //unauthorized access
                return Err(ProgramError::Custom(0x02));
            }

            let contract_data = ContractData{
                min_period,
                reward_period,
            };
            contract_data.serialize(&mut &mut pda.data.borrow_mut()[..])?;
        }
    };
        
    Ok(())
}


