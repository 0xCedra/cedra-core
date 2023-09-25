spec aptos_framework::object {

    spec module {
        pragma aborts_if_is_strict;
    }

    spec fun spec_exists_at<T: key>(object: address): bool;

    spec exists_at<T: key>(object: address): bool {
        pragma opaque;
        ensures [abstract] result == spec_exists_at<T>(object);
    }

    spec address_to_object<T: key>(object: address): Object<T> {
        aborts_if !exists<ObjectCore>(object);
        aborts_if !spec_exists_at<T>(object);
        ensures result == Object<T> { inner: object };
    }

    spec create_object(owner_address: address): ConstructorRef{
        use std::features;
        pragma aborts_if_is_partial;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if !features::spec_is_enabled(features::APTOS_UNIQUE_IDENTIFIERS);
        aborts_if exists<ObjectCore>(unique_address);

        ensures exists<ObjectCore>(unique_address);
        ensures global<ObjectCore>(unique_address) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: owner_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: unique_address,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: unique_address, can_delete: true };
    }

    spec create_sticky_object(owner_address: address): ConstructorRef{
        use std::features;
        pragma aborts_if_is_partial;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if !features::spec_is_enabled(features::APTOS_UNIQUE_IDENTIFIERS);
        aborts_if exists<ObjectCore>(unique_address);

        ensures exists<ObjectCore>(unique_address);
        ensures global<ObjectCore>(unique_address) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: owner_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: unique_address,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: unique_address, can_delete: false };
    }

    spec create_object_address(source: &address, seed: vector<u8>): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_object_address(source, seed);
    }

    spec create_user_derived_object_address(source: address, derive_from: address): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_user_derived_object_address(source, derive_from);
    }

    spec create_guid_object_address(source: address, creation_num: u64): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_guid_object_address(source, creation_num);
    }

    spec object_address<T: key>(object: &Object<T>): address {
        aborts_if false;
        ensures result == object.inner;
    }

    spec convert<X: key, Y: key>(object: Object<X>): Object<Y> {
        aborts_if !exists<ObjectCore>(object.inner);
        aborts_if !spec_exists_at<Y>(object.inner);
        ensures result == Object<Y> { inner: object.inner };
    }

    spec create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let obj_addr = spec_create_object_address(creator_address, seed);
        aborts_if exists<ObjectCore>(obj_addr);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: false };
    }

    spec create_user_derived_object(creator_address: address, derive_ref: &DeriveRef): ConstructorRef {
        let obj_addr = spec_create_user_derived_object_address(creator_address, derive_ref.self);
        aborts_if exists<ObjectCore>(obj_addr);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: false };
    }

    spec create_object_from_account(creator: &signer): ConstructorRef {
        aborts_if !exists<account::Account>(signer::address_of(creator));
        //Guid properties
        let object_data = global<account::Account>(signer::address_of(creator));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        aborts_if object_data.guid_creation_num + 1 >= account::MAX_GUID_CREATION_NUM;
        let creation_num = object_data.guid_creation_num;
        let addr = signer::address_of(creator);

        let guid = guid::GUID {
            id: guid::ID {
                creation_num,
                addr,
            }
        };

        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures global<account::Account>(addr).guid_creation_num == old(global<account::Account>(addr)).guid_creation_num + 1;
        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: addr,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
    }

    spec create_object_from_object(creator: &signer): ConstructorRef{
        aborts_if !exists<ObjectCore>(signer::address_of(creator));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(creator));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        let creation_num = object_data.guid_creation_num;
        let addr = signer::address_of(creator);

        let guid = guid::GUID {
            id: guid::ID {
                creation_num,
                addr,
            }
        };

        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures global<ObjectCore>(addr).guid_creation_num == old(global<ObjectCore>(addr)).guid_creation_num + 1;
        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: addr,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
    }

    spec create_object_from_guid(creator_address: address, guid: guid::GUID): ConstructorRef {
        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr,
                        }
                    }
                }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
    }

    spec create_object_internal(
        creator_address: address,
        object: address,
        can_delete: bool,
    ): ConstructorRef {
        // property 1: Creating an object twice on the same address must never occur.
        aborts_if exists<ObjectCore>(object);
        ensures exists<ObjectCore>(object);
        // property 6: Object addresses must not overlap with other addresses in different domains.
        ensures global<ObjectCore>(object).guid_creation_num ==  INIT_GUID_CREATION_NUM + 1;
        ensures result == ConstructorRef { self: object, can_delete };
    }

    spec generate_delete_ref(ref: &ConstructorRef): DeleteRef {
        aborts_if !ref.can_delete;
        ensures result == DeleteRef { self: ref.self };
    }

    spec disable_ungated_transfer(ref: &TransferRef) {
        aborts_if !exists<ObjectCore>(ref.self);
        ensures global<ObjectCore>(ref.self).allow_ungated_transfer == false;
    }

    spec object_from_constructor_ref<T: key>(ref: &ConstructorRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !spec_exists_at<T>(ref.self);
        ensures result == Object<T> { inner: ref.self };
    }

    spec create_guid(object: &signer): guid::GUID{
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;

        ensures result == guid::GUID {
            id: guid::ID {
                creation_num: object_data.guid_creation_num,
                addr: signer::address_of(object)
            }
        };
    }

    spec new_event_handle<T: drop + store>(
        object: &signer,
    ): event::EventHandle<T>{
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;

        let guid = guid::GUID {
            id: guid::ID {
                creation_num: object_data.guid_creation_num,
                addr: signer::address_of(object)
            }
        };
        ensures result == event::EventHandle<T> {
            counter: 0,
            guid,
        };
    }

    spec object_from_delete_ref<T: key>(ref: &DeleteRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !spec_exists_at<T>(ref.self);
        ensures result == Object<T> { inner: ref.self };
    }

    spec delete(ref: DeleteRef) {
        aborts_if !exists<ObjectCore>(ref.self);
        ensures !exists<ObjectCore>(ref.self);
    }

    spec enable_ungated_transfer(ref: &TransferRef) {
        aborts_if !exists<ObjectCore>(ref.self);
        ensures global<ObjectCore>(ref.self).allow_ungated_transfer == true;
    }

    spec generate_linear_transfer_ref(ref: &TransferRef): LinearTransferRef {
        aborts_if !exists<ObjectCore>(ref.self);
        let owner = global<ObjectCore>(ref.self).owner;
        ensures result == LinearTransferRef {
            self: ref.self,
            owner,
        };
    }

    spec transfer_with_ref(ref: LinearTransferRef, to: address){
        let object = global<ObjectCore>(ref.self);
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if object.owner != ref.owner;
        ensures global<ObjectCore>(ref.self).owner == to;
    }

    spec transfer_call(
        owner: &signer,
        object: address,
        to: address,
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
    }

    spec transfer<T: key>(
        owner: &signer,
        object: Object<T>,
        to: address,
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
        // property 3: The 'indirect' owner of an object may transfer the object.
        let post new_owner_address = global<ObjectCore>(object_address).owner;
        ensures owner_address != object_address ==> new_owner_address == to;
    }

    spec transfer_raw(
        owner: &signer,
        object: address,
        to: address,
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
    }

    spec transfer_to_object<O: key, T: key> (
        owner: &signer,
        object: Object<O>,
        to: Object<T>,
    ){
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
    }

    spec verify_ungated_and_descendant(owner: address, destination: address) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        aborts_if !exists<ObjectCore>(destination);
        aborts_if !global<ObjectCore>(destination).allow_ungated_transfer;
    }

    spec ungated_transfer_allowed<T: key>(object: Object<T>): bool {
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == global<ObjectCore>(object.inner).allow_ungated_transfer;
    }

    spec is_owner<T: key>(object: Object<T>, owner: address): bool{
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == (global<ObjectCore>(object.inner).owner == owner);
    }

    spec owner<T: key>(object: Object<T>): address{
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == global<ObjectCore>(object.inner).owner;
    }

    spec owns<T: key>(object: Object<T>, owner: address): bool {
        let current_address_0 = object.inner;
        let object_0 = global<ObjectCore>(current_address_0);
        let current_address = object_0.owner;
        aborts_if object.inner != owner && !exists<ObjectCore>(object.inner);
        ensures current_address_0 == owner ==> result == true;
    }

    // Helper function
    spec fun spec_create_object_address(source: address, seed: vector<u8>): address;

    spec fun spec_create_user_derived_object_address(source: address, derive_from: address): address;

    spec fun spec_create_guid_object_address(source: address, creation_num: u64): address;

}
