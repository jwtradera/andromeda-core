use crate::{
    contract::{execute, instantiate, query},
    state::{add_pathname, resolve_pathname, PathInfo, ADDRESS_LIBRARY, ADDRESS_USERNAME, USERS},
};

use andromeda_std::{
    amp::AndrAddr,
    os::{
        kernel::{ExecuteMsg as KernelExecuteMsg, InternalMsg},
        vfs::{ExecuteMsg, InstantiateMsg},
    },
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_FAKE_KERNEL_CONTRACT, MOCK_KERNEL_CONTRACT,
    },
};
use andromeda_std::{error::ContractError, os::vfs::QueryMsg};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, WasmMsg,
};

fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) {
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };

    let res = instantiate(deps, env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env, info)
}

#[test]
fn test_register_user() {
    let mut deps = mock_dependencies_custom(&[]);
    let username = "user1";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: None,
    };
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());
    execute(deps.as_mut(), env, info, msg).unwrap();

    let saved = USERS.load(deps.as_ref().storage, username).unwrap();
    assert_eq!(saved, sender)
}

#[test]
fn test_register_user_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let username = "user1";
    let sender = "sender";
    let occupier = "occupier";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: None,
    };
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());
    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(occupier))
        .unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {})
}

#[test]
fn test_register_user_already_registered() {
    let mut deps = mock_dependencies_custom(&[]);
    let username = "user1";
    let new_username = "user2";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: new_username.to_string(),
        address: None,
    };
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(sender))
        .unwrap();

    execute(deps.as_mut(), env, info, msg).unwrap();
    let addr = USERS.load(deps.as_ref().storage, new_username).unwrap();
    assert_eq!(addr, sender);
    let username = ADDRESS_USERNAME
        .load(deps.as_ref().storage, sender)
        .unwrap();
    assert_eq!(username, new_username)
}

#[test]
fn test_register_user_foreign_chain() {
    let mut deps = mock_dependencies_custom(&[]);
    let username = "user1";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        kernel_address: MOCK_FAKE_KERNEL_CONTRACT.to_string(),
        owner: None,
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: None,
    };
    let err = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: Some(Addr::unchecked(sender)),
    };
    let info = mock_info(MOCK_FAKE_KERNEL_CONTRACT, &[]);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let addr = USERS.load(deps.as_ref().storage, username).unwrap();
    assert_eq!(addr, sender);

    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: None,
    };
    let info = mock_info(MOCK_FAKE_KERNEL_CONTRACT, &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_register_user_cross_chain() {
    let mut deps = mock_dependencies_custom(&[]);
    let username = "user1";
    let sender = "sender";
    let foreign_address = "address";
    let chain = "chain";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let msg = ExecuteMsg::RegisterUserCrossChain {
        chain: chain.to_string(),
        address: foreign_address.to_string(),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_err());

    ADDRESS_USERNAME
        .save(deps.as_mut().storage, sender, &username.to_string())
        .unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 1);

    let expected = KernelExecuteMsg::Internal(InternalMsg::RegisterUserCrossChain {
        username: username.to_string(),
        address: foreign_address.to_string(),
        chain: chain.to_string(),
    });

    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_KERNEL_CONTRACT.to_string(),
            msg: to_binary(&expected).unwrap(),
            funds: vec![],
        })
    );
}

#[test]
fn test_add_path() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let component_name = "f1";
    let sender = "sender";
    let component_addr = Addr::unchecked("f1addr");
    let info = mock_info(sender, &[]);
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let msg = ExecuteMsg::AddPath {
        name: component_name.to_string(),
        address: component_addr.clone(),
        parent_address: None,
    };

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(sender))
        .unwrap();

    let path = format!("/home/{username}/{component_name}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, component_addr);

    let component_name_two = "component_two";
    let component_addr_two = Addr::unchecked("component_two_addr");
    let msg = ExecuteMsg::AddPath {
        name: component_name_two.to_string(),
        address: component_addr_two.clone(),
        parent_address: Some(AndrAddr::from_string(component_addr.clone())),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let path = format!("/home/{username}/{component_name}/{component_name_two}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, component_addr_two);

    let info = mock_info("not_the_owner", &[]);
    let component_name_two = "component_two";
    let component_addr_two = Addr::unchecked("component_two_addr");
    let msg = ExecuteMsg::AddPath {
        name: component_name_two.to_string(),
        address: component_addr_two,
        parent_address: Some(AndrAddr::from_string(component_addr)),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
fn test_add_symlink() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let component_name = "f1";
    let sender = "sender";
    let component_addr = Addr::unchecked("f1addr");
    let info = mock_info(sender, &[]);
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let msg = ExecuteMsg::AddPath {
        name: component_name.to_string(),
        address: component_addr.clone(),
        parent_address: None,
    };

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(sender))
        .unwrap();

    let path = format!("/home/{username}/{component_name}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, component_addr);

    let symlink_name = "symlink";
    let msg = ExecuteMsg::AddSymlink {
        name: symlink_name.to_string(),
        symlink: AndrAddr::from_string(format!("/home/{username}/{component_name}")),
        parent_address: None,
    };

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let path = format!("/home/{username}/{symlink_name}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, component_addr);

    let symlink_two_name = "symlink_two";
    let msg = ExecuteMsg::AddSymlink {
        name: symlink_two_name.to_string(),
        symlink: AndrAddr::from_string(format!("ibc://chain/home/{username}/{component_name}")),
        parent_address: None,
    };

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let path = format!("/home/{username}/{symlink_two_name}");

    let err = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidAddress {});

    let symlink_three_name = "symlink_three";
    let msg = ExecuteMsg::AddSymlink {
        name: symlink_three_name.to_string(),
        symlink: AndrAddr::from_string(format!("/home/{username}/{symlink_name}")),
        parent_address: None,
    };

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let path = format!("/home/{username}/{symlink_three_name}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, component_addr);

    let symlink_four_name = "symlink_four";
    let msg = ExecuteMsg::AddSymlink {
        name: symlink_four_name.to_string(),
        symlink: AndrAddr::from_string(format!("/home/{username}/{component_name}")),
        parent_address: Some(AndrAddr::from_string(format!(
            "/home/{username}/{component_name}/"
        ))),
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    let path = format!("/home/{username}/{component_name}/{symlink_four_name}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, component_addr);
}

#[test]
fn test_add_parent_path() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let user_address = Addr::unchecked("useraddr");
    let component_name = "f1";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::AddParentPath {
        name: component_name.to_string(),
        parent_address: AndrAddr::from_string(format!("/home/{user_address}")),
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    USERS
        .save(deps.as_mut().storage, username, &user_address)
        .unwrap();

    let path = format!("/home/{username}/{component_name}");

    let resolved_addr = resolve_pathname(
        deps.as_ref().storage,
        deps.as_ref().api,
        AndrAddr::from_string(path),
    )
    .unwrap();

    assert_eq!(resolved_addr, sender)
}

/**
 * This test tries to override existing vfs path using add parent path method.
 * Here user_one will set his address as identifier in vfs. Another user user_two
 * will try to override this field with his address so that he can get the benefits
 * like splitter funds etc without user_one authorisation.
 * Add Parent path has a protection to prevent such type of override and this test is
 * VERY IMPORTANT from security perspective
 */
#[test]
fn test_override_add_parent_path() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let user_address = Addr::unchecked("user_one");
    let component_name = "identifier";
    let info = mock_info(user_address.as_str(), &[]);
    let msg = ExecuteMsg::AddParentPath {
        name: component_name.to_string(),
        parent_address: AndrAddr::from_string(format!("/home/{user_address}")),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Try to override above address with your address
    let info = mock_info("user_two", &[]);
    let msg = ExecuteMsg::AddParentPath {
        name: component_name.to_string(),
        parent_address: AndrAddr::from_string(format!("/home/{user_address}")),
    };

    // This will error, user_two is trying to add his address as identifier for /user_one/identifier vfs path
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn test_get_username() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let username = "u1";
    let sender = "sender";

    ADDRESS_USERNAME
        .save(deps.as_mut().storage, sender, &username.to_string())
        .unwrap();

    let query_msg = QueryMsg::GetUsername {
        address: Addr::unchecked(sender),
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: String = from_binary(&res).unwrap();

    assert_eq!(val, username);

    let unregistered_addr = "notregistered";
    let query_msg = QueryMsg::GetUsername {
        address: Addr::unchecked(unregistered_addr),
    };

    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: String = from_binary(&res).unwrap();

    assert_eq!(val, unregistered_addr);
}

#[test]
fn test_get_library() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let lib_name = "l1";
    let sender = "sender";

    ADDRESS_LIBRARY
        .save(deps.as_mut().storage, sender, &lib_name.to_string())
        .unwrap();

    let query_msg = QueryMsg::GetLibrary {
        address: Addr::unchecked(sender),
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: String = from_binary(&res).unwrap();

    assert_eq!(val, lib_name);

    let unregistered_addr = "notregistered";
    let query_msg = QueryMsg::GetLibrary {
        address: Addr::unchecked(unregistered_addr),
    };

    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: String = from_binary(&res).unwrap();

    assert_eq!(val, unregistered_addr);
}

#[test]
fn test_get_subdir() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let sender = Addr::unchecked("sender");
    let env = mock_env();
    let root_paths = vec![
        PathInfo {
            name: "f1".to_string(),
            address: Addr::unchecked("f1addr"),
            parent_address: sender.clone(),
            symlink: None,
        },
        PathInfo {
            name: "f2".to_string(),
            address: Addr::unchecked("f2addr"),
            parent_address: sender.clone(),
            symlink: None,
        },
    ];
    let sub_paths = vec![
        PathInfo {
            name: "sub1".to_string(),
            address: Addr::unchecked("sub1addr"),
            parent_address: root_paths[0].address.clone(),
            symlink: None,
        },
        PathInfo {
            name: "sub2".to_string(),
            address: Addr::unchecked("sub2addr"),
            parent_address: root_paths[0].address.clone(),
            symlink: None,
        },
    ];

    USERS
        .save(deps.as_mut().storage, username, &sender)
        .unwrap();

    // Add all root components
    for path in root_paths.clone() {
        let _ = add_pathname(
            deps.as_mut().storage,
            sender.clone(),
            path.name,
            path.address,
        );
    }

    for path in sub_paths.clone() {
        let _ = add_pathname(
            deps.as_mut().storage,
            path.parent_address.clone(),
            path.name,
            path.address,
        );
    }

    for path in root_paths.clone() {
        let path_name = format!("/home/{username}/{name}", name = path.name);
        let resolved_addr = resolve_pathname(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(path_name.clone()),
        );
        assert!(resolved_addr.is_ok(), "{path_name} not found");
        assert_eq!(resolved_addr.unwrap(), path.address)
    }

    let query_msg = QueryMsg::SubDir {
        path: AndrAddr::from_string(format!("/home/{username}")),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: Vec<PathInfo> = from_binary(&res).unwrap();
    assert_eq!(val, root_paths);

    let subdir = &root_paths[0].name;
    let query_msg = QueryMsg::SubDir {
        path: AndrAddr::from_string(format!("/home/{username}/{subdir}")),
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: Vec<PathInfo> = from_binary(&res).unwrap();
    assert_eq!(val, sub_paths);
}

#[test]
fn test_get_paths() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let sender = Addr::unchecked("sender");
    let env = mock_env();
    let root_paths = vec![
        PathInfo {
            name: "f1".to_string(),
            address: Addr::unchecked("f1addr"),
            parent_address: sender.clone(),
            symlink: None,
        },
        PathInfo {
            name: "f2".to_string(),
            address: Addr::unchecked("f2addr"),
            parent_address: sender.clone(),
            symlink: None,
        },
    ];
    let sub_paths = vec![
        PathInfo {
            name: "sub1".to_string(),
            address: Addr::unchecked("sub1addr"),
            parent_address: root_paths[0].address.clone(),
            symlink: None,
        },
        PathInfo {
            name: "sub2".to_string(),
            address: Addr::unchecked("sub2addr"),
            parent_address: root_paths[0].address.clone(),
            symlink: None,
        },
    ];

    USERS
        .save(deps.as_mut().storage, username, &sender)
        .unwrap();
    ADDRESS_USERNAME
        .save(
            deps.as_mut().storage,
            sender.as_str(),
            &username.to_string(),
        )
        .unwrap();

    // Add all root components
    for path in root_paths.clone() {
        let _ = add_pathname(
            deps.as_mut().storage,
            sender.clone(),
            path.name,
            path.address.clone(),
        );
        for sub_path in sub_paths.clone() {
            let _ = add_pathname(
                deps.as_mut().storage,
                path.address.clone(),
                sub_path.name,
                sub_path.address,
            );
        }
    }

    for path in root_paths {
        let path_name = format!("/home/{username}/{name}", name = path.name);
        let resolved_addr = resolve_pathname(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(path_name.clone()),
        );
        assert!(resolved_addr.is_ok(), "{path_name} not found");
        assert_eq!(resolved_addr.unwrap(), path.address)
    }

    let query_msg = QueryMsg::Paths {
        addr: sub_paths[0].address.clone(),
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: Vec<String> = from_binary(&res).unwrap();
    assert_eq!(val.len(), 2);
}
