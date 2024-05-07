//! Parity tests

use crate::utils::{inspect, print_traces};
use alloy_primitives::{address, hex, Address, U256};
use alloy_rpc_types::TransactionInfo;
use alloy_rpc_types_trace::parity::{Action, SelfdestructAction};
use revm::{
    db::{CacheDB, EmptyDB},
    interpreter::CreateScheme,
    primitives::{
        AccountInfo, BlockEnv, CfgEnv, CfgEnvWithHandlerCfg, EnvWithHandlerCfg, ExecutionResult,
        HandlerCfg, Output, SpecId, TransactTo, TxEnv,
    },
    DatabaseCommit,
};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

#[test]
fn test_parity_selfdestruct_london() {
    test_parity_selfdestruct(SpecId::LONDON);
}

#[test]
fn test_parity_selfdestruct_cancun() {
    test_parity_selfdestruct(SpecId::CANCUN);
}

fn test_parity_selfdestruct(spec_id: SpecId) {
    /*
    contract DummySelfDestruct {
        constructor() payable {}
        function close() public {
            selfdestruct(payable(msg.sender));
        }
    }
    */

    // simple contract that selfdestructs when a function is called
    let code = hex!("608080604052606b908160108239f3fe6004361015600c57600080fd5b6000803560e01c6343d726d614602157600080fd5b346032578060031936011260325733ff5b80fdfea2646970667358221220f393fc6be90126d52315ccd38ae6608ac4fd5bef4c59e119e280b2a2b149d0dc64736f6c63430008190033");

    let deployer = address!("341348115259a8bf69f1f50101c227fced83bac6");
    let value = U256::from(69);

    let mut db = CacheDB::new(EmptyDB::default());
    db.insert_account_info(deployer, AccountInfo { balance: value, ..Default::default() });

    let cfg = CfgEnvWithHandlerCfg::new(CfgEnv::default(), HandlerCfg::new(spec_id));
    let env = EnvWithHandlerCfg::new_with_cfg_env(
        cfg.clone(),
        BlockEnv::default(),
        TxEnv {
            caller: deployer,
            gas_limit: 1000000,
            transact_to: TransactTo::Create(CreateScheme::Create),
            data: code.into(),
            value,
            ..Default::default()
        },
    );

    let mut insp = TracingInspector::new(TracingInspectorConfig::default_parity());

    let (res, _) = inspect(&mut db, env, &mut insp).unwrap();
    let contract_address = match res.result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Create(_, addr) => addr.unwrap(),
            _ => panic!("Create failed"),
        },
        _ => panic!("Execution failed"),
    };
    db.commit(res.state);

    let mut insp = TracingInspector::new(TracingInspectorConfig::default_parity());

    let env = EnvWithHandlerCfg::new_with_cfg_env(
        cfg,
        BlockEnv::default(),
        TxEnv {
            caller: deployer,
            gas_limit: 1000000,
            transact_to: TransactTo::Call(contract_address),
            data: hex!("43d726d6").into(),
            ..Default::default()
        },
    );

    let (res, _) = inspect(&mut db, env, &mut insp).unwrap();
    assert!(res.result.is_success(), "{res:#?}");

    // TODO: Transfer still happens in Cancun, but this is not reflected in the trace.
    let (expected_value, expected_target) =
        if spec_id < SpecId::CANCUN { (value, Some(deployer)) } else { (U256::ZERO, None) };

    {
        assert_eq!(insp.get_traces().nodes().len(), 1);
        let node = &insp.get_traces().nodes()[0];
        assert!(node.is_selfdestruct(), "{node:#?}");
        assert_eq!(node.trace.address, contract_address);
        assert_eq!(node.trace.selfdestruct_refund_target, expected_target);
        assert_eq!(node.trace.value, expected_value);
    }

    let traces = insp
        .with_transaction_gas_used(res.result.gas_used())
        .into_parity_builder()
        .into_localized_transaction_traces(TransactionInfo::default());

    assert_eq!(traces.len(), 2);
    assert_eq!(
        traces[1].trace.action,
        Action::Selfdestruct(SelfdestructAction {
            address: contract_address,
            refund_address: expected_target.unwrap_or_default(),
            balance: expected_value,
        })
    );
}

// Minimal example of <https://etherscan.io/tx/0xd81725127173cf1095a722cbaec118052e2626ddb914d61967fb4bf117969be0>
#[test]
fn test_parity_constructor_selfdestruct() {
    // simple contract that selfdestructs when a function is called

    /*
    contract DummySelfDestruct {
        function close() public {
            new Noop();
        }
    }
    contract Noop {
        constructor() {
            selfdestruct(payable(msg.sender));
        }
    }
    */

    let code = hex!("6080604052348015600f57600080fd5b5060b48061001e6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c806343d726d614602d575b600080fd5b60336035565b005b604051603f90605e565b604051809103906000f080158015605a573d6000803e3d6000fd5b5050565b60148061006b8339019056fe6080604052348015600f57600080fd5b5033fffea264697066735822122087fcd1ed364913e41107ea336facf7b7f5972695b3e3abcf55dbb2452e124ea964736f6c634300080d0033");

    let deployer = Address::ZERO;

    let mut db = CacheDB::new(EmptyDB::default());

    let cfg = CfgEnvWithHandlerCfg::new(CfgEnv::default(), HandlerCfg::new(SpecId::LONDON));

    let env = EnvWithHandlerCfg::new_with_cfg_env(
        cfg.clone(),
        BlockEnv::default(),
        TxEnv {
            caller: deployer,
            gas_limit: 1000000,
            transact_to: TransactTo::Create(CreateScheme::Create),
            data: code.into(),
            ..Default::default()
        },
    );

    let mut insp = TracingInspector::new(TracingInspectorConfig::default_parity());
    let (res, _) = inspect(&mut db, env, &mut insp).unwrap();
    let addr = match res.result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Create(_, addr) => addr.unwrap(),
            _ => panic!("Create failed"),
        },
        _ => panic!("Execution failed"),
    };
    db.commit(res.state);
    print_traces(&insp);

    let mut insp = TracingInspector::new(TracingInspectorConfig::default_parity());

    let env = EnvWithHandlerCfg::new_with_cfg_env(
        cfg,
        BlockEnv::default(),
        TxEnv {
            caller: deployer,
            gas_limit: 1000000,
            transact_to: TransactTo::Call(addr),
            data: hex!("43d726d6").into(),
            ..Default::default()
        },
    );

    let (res, _) = inspect(&mut db, env, &mut insp).unwrap();
    assert!(res.result.is_success());
    print_traces(&insp);

    let traces = insp
        .with_transaction_gas_used(res.result.gas_used())
        .into_parity_builder()
        .into_localized_transaction_traces(TransactionInfo::default());

    assert_eq!(traces.len(), 3);
    assert!(traces[1].trace.action.is_create());
    assert_eq!(traces[1].trace.trace_address, vec![0]);
    assert_eq!(traces[1].trace.subtraces, 1);
    assert!(traces[2].trace.action.is_selfdestruct());
    assert_eq!(traces[2].trace.trace_address, vec![0, 0]);
}
