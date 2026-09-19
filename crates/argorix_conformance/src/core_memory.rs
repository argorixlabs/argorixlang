//! Safe Rust prototype of Argorix Core M1/H1 for ESP-005.
//!
//! This is transition evidence, not the independent runtime. It uses no unsafe
//! code and keeps the canonical handle encoding independent from Rust layout.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const HANDLE_SIZE: usize = 48;
pub const NODE_SIZE: usize = 120;
pub const TYPE_BYTE: u32 = 1;
pub const TYPE_NODE: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Permission {
    Read = 1,
    ReadWrite = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Handle {
    pub arena_id: u64,
    pub arena_epoch: u64,
    pub slot: u32,
    pub allocation_generation: u32,
    pub offset: u64,
    pub length: u64,
    pub type_id: u32,
    pub permission: Permission,
}

impl Handle {
    pub fn encode(self) -> [u8; HANDLE_SIZE] {
        let mut out = [0_u8; HANDLE_SIZE];
        out[0..8].copy_from_slice(&self.arena_id.to_le_bytes());
        out[8..16].copy_from_slice(&self.arena_epoch.to_le_bytes());
        out[16..20].copy_from_slice(&self.slot.to_le_bytes());
        out[20..24].copy_from_slice(&self.allocation_generation.to_le_bytes());
        out[24..32].copy_from_slice(&self.offset.to_le_bytes());
        out[32..40].copy_from_slice(&self.length.to_le_bytes());
        out[40..44].copy_from_slice(&self.type_id.to_le_bytes());
        out[44] = self.permission as u8;
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MemoryError> {
        if bytes.len() != HANDLE_SIZE || bytes[45..48] != [0, 0, 0] {
            return Err(MemoryError::InvalidHandle);
        }
        let permission = match bytes[44] {
            1 => Permission::Read,
            2 => Permission::ReadWrite,
            _ => return Err(MemoryError::InvalidHandle),
        };
        Ok(Self {
            arena_id: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            arena_epoch: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            slot: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            allocation_generation: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            offset: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            length: u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
            type_id: u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            permission,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorrowKind {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BorrowRecord {
    slot: u32,
    generation: u32,
    kind: BorrowKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Allocation {
    generation: u32,
    type_id: u32,
    element_size: u64,
    length: u64,
    mutable: bool,
    live: bool,
    data: Vec<u8>,
    read_borrows: u32,
    write_borrow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena {
    id: u64,
    epoch: u64,
    owner: u64,
    byte_limit: u64,
    used_bytes: u64,
    peak_used_bytes: u64,
    max_slots: usize,
    max_tokens: usize,
    released: bool,
    allocations: Vec<Allocation>,
    borrows: BTreeMap<u64, BorrowRecord>,
    next_token: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Error)]
pub enum MemoryError {
    #[error("InvalidHandle")]
    InvalidHandle,
    #[error("UseAfterFree")]
    UseAfterFree,
    #[error("ArenaReleased")]
    ArenaReleased,
    #[error("OutOfBounds")]
    OutOfBounds,
    #[error("PermissionDenied")]
    PermissionDenied,
    #[error("BorrowConflict")]
    BorrowConflict,
    #[error("InvalidBorrow")]
    InvalidBorrow,
    #[error("NotOwner")]
    NotOwner,
    #[error("InvalidOwner")]
    InvalidOwner,
    #[error("NotRoot")]
    NotRoot,
    #[error("DoubleFree")]
    DoubleFree,
    #[error("Overflow")]
    Overflow,
    #[error("OutOfMemory")]
    OutOfMemory,
    #[error("ResourceLimit")]
    ResourceLimit,
    #[error("TypeMismatch")]
    TypeMismatch,
    #[error("DepthLimit")]
    DepthLimit,
    #[error("CapabilityMissing")]
    CapabilityMissing,
    #[error("ProfileMismatch")]
    ProfileMismatch,
    #[error("OperationDenied")]
    OperationDenied,
}

impl Arena {
    pub fn new(
        id: u64,
        owner: u64,
        byte_limit: u64,
        max_slots: usize,
        max_tokens: usize,
    ) -> Result<Self, MemoryError> {
        if id == 0 || owner == 0 || byte_limit == 0 || max_slots == 0 || max_tokens == 0 {
            return Err(MemoryError::ResourceLimit);
        }
        Ok(Self {
            id,
            epoch: 1,
            owner,
            byte_limit,
            used_bytes: 0,
            peak_used_bytes: 0,
            max_slots,
            max_tokens,
            released: false,
            allocations: Vec::new(),
            borrows: BTreeMap::new(),
            next_token: 1,
        })
    }

    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    pub fn peak_used_bytes(&self) -> u64 {
        self.peak_used_bytes
    }

    pub fn owner(&self) -> u64 {
        self.owner
    }

    fn require_owner(&self, owner: u64) -> Result<(), MemoryError> {
        if owner == 0 {
            Err(MemoryError::InvalidOwner)
        } else if owner != self.owner {
            Err(MemoryError::NotOwner)
        } else {
            Ok(())
        }
    }

    pub fn alloc(
        &mut self,
        owner: u64,
        type_id: u32,
        length: u64,
        element_size: u64,
        mutable: bool,
    ) -> Result<Handle, MemoryError> {
        self.require_owner(owner)?;
        if self.released {
            return Err(MemoryError::ArenaReleased);
        }
        if type_id == 0 || element_size == 0 {
            return Err(MemoryError::TypeMismatch);
        }
        let bytes = length
            .checked_mul(element_size)
            .ok_or(MemoryError::Overflow)?;
        let next_used = self
            .used_bytes
            .checked_add(bytes)
            .ok_or(MemoryError::Overflow)?;
        if next_used > self.byte_limit {
            return Err(MemoryError::OutOfMemory);
        }
        let byte_len: usize = bytes.try_into().map_err(|_| MemoryError::ResourceLimit)?;
        let reusable = self
            .allocations
            .iter()
            .position(|allocation| !allocation.live);
        let slot = if let Some(slot) = reusable {
            slot
        } else {
            if self.allocations.len() >= self.max_slots {
                return Err(MemoryError::ResourceLimit);
            }
            self.allocations.push(Allocation {
                generation: 1,
                type_id,
                element_size,
                length,
                mutable,
                live: false,
                data: Vec::new(),
                read_borrows: 0,
                write_borrow: false,
            });
            self.allocations.len() - 1
        };
        let generation = self.allocations[slot].generation;
        self.allocations[slot] = Allocation {
            generation,
            type_id,
            element_size,
            length,
            mutable,
            live: true,
            data: vec![0; byte_len],
            read_borrows: 0,
            write_borrow: false,
        };
        self.used_bytes = next_used;
        self.peak_used_bytes = self.peak_used_bytes.max(next_used);
        Ok(Handle {
            arena_id: self.id,
            arena_epoch: self.epoch,
            slot: slot as u32,
            allocation_generation: generation,
            offset: 0,
            length,
            type_id,
            permission: if mutable {
                Permission::ReadWrite
            } else {
                Permission::Read
            },
        })
    }

    fn allocation(&self, handle: Handle) -> Result<&Allocation, MemoryError> {
        if self.released {
            return Err(MemoryError::ArenaReleased);
        }
        if handle.arena_id != self.id || handle.arena_epoch != self.epoch {
            return Err(MemoryError::InvalidHandle);
        }
        let allocation = self
            .allocations
            .get(handle.slot as usize)
            .ok_or(MemoryError::InvalidHandle)?;
        if !allocation.live || allocation.generation != handle.allocation_generation {
            return Err(MemoryError::UseAfterFree);
        }
        if allocation.type_id != handle.type_id {
            return Err(MemoryError::TypeMismatch);
        }
        let end = handle
            .offset
            .checked_add(handle.length)
            .ok_or(MemoryError::Overflow)?;
        if end > allocation.length {
            return Err(MemoryError::OutOfBounds);
        }
        Ok(allocation)
    }

    fn byte_range(
        &self,
        handle: Handle,
        index: u64,
    ) -> Result<std::ops::Range<usize>, MemoryError> {
        let allocation = self.allocation(handle)?;
        if index >= handle.length {
            return Err(MemoryError::OutOfBounds);
        }
        let element = handle
            .offset
            .checked_add(index)
            .ok_or(MemoryError::Overflow)?;
        let start = element
            .checked_mul(allocation.element_size)
            .ok_or(MemoryError::Overflow)?;
        let end = start
            .checked_add(allocation.element_size)
            .ok_or(MemoryError::Overflow)?;
        Ok(
            usize::try_from(start).map_err(|_| MemoryError::ResourceLimit)?
                ..usize::try_from(end).map_err(|_| MemoryError::ResourceLimit)?,
        )
    }

    pub fn slice(
        &self,
        handle: Handle,
        start: u64,
        length: u64,
        permission: Permission,
    ) -> Result<Handle, MemoryError> {
        self.allocation(handle)?;
        let end = start.checked_add(length).ok_or(MemoryError::Overflow)?;
        if end > handle.length {
            return Err(MemoryError::OutOfBounds);
        }
        if permission == Permission::ReadWrite && handle.permission != Permission::ReadWrite {
            return Err(MemoryError::PermissionDenied);
        }
        Ok(Handle {
            offset: handle
                .offset
                .checked_add(start)
                .ok_or(MemoryError::Overflow)?,
            length,
            permission,
            ..handle
        })
    }

    pub fn read_element(&self, handle: Handle, index: u64) -> Result<Vec<u8>, MemoryError> {
        let range = self.byte_range(handle, index)?;
        let allocation = self.allocation(handle)?;
        if allocation.write_borrow {
            return Err(MemoryError::BorrowConflict);
        }
        Ok(allocation.data[range].to_vec())
    }

    pub fn write_element(
        &mut self,
        handle: Handle,
        index: u64,
        value: &[u8],
    ) -> Result<(), MemoryError> {
        let range = self.byte_range(handle, index)?;
        if handle.permission != Permission::ReadWrite {
            return Err(MemoryError::PermissionDenied);
        }
        let allocation = self.allocation(handle)?;
        if !allocation.mutable {
            return Err(MemoryError::PermissionDenied);
        }
        if allocation.read_borrows > 0 || allocation.write_borrow {
            return Err(MemoryError::BorrowConflict);
        }
        if value.len() != range.len() {
            return Err(MemoryError::TypeMismatch);
        }
        self.allocations[handle.slot as usize].data[range].copy_from_slice(value);
        Ok(())
    }

    pub fn borrow_read(&mut self, handle: Handle) -> Result<u64, MemoryError> {
        let allocation = self.allocation(handle)?;
        if allocation.write_borrow {
            return Err(MemoryError::BorrowConflict);
        }
        if self.borrows.len() >= self.max_tokens {
            return Err(MemoryError::ResourceLimit);
        }
        let token = self.next_token;
        self.next_token = self
            .next_token
            .checked_add(1)
            .ok_or(MemoryError::Overflow)?;
        let reads = self.allocations[handle.slot as usize]
            .read_borrows
            .checked_add(1)
            .ok_or(MemoryError::ResourceLimit)?;
        self.allocations[handle.slot as usize].read_borrows = reads;
        self.borrows.insert(
            token,
            BorrowRecord {
                slot: handle.slot,
                generation: handle.allocation_generation,
                kind: BorrowKind::Read,
            },
        );
        Ok(token)
    }

    pub fn borrow_write(&mut self, handle: Handle) -> Result<u64, MemoryError> {
        let allocation = self.allocation(handle)?;
        if handle.permission != Permission::ReadWrite || !allocation.mutable {
            return Err(MemoryError::PermissionDenied);
        }
        if allocation.read_borrows > 0 || allocation.write_borrow {
            return Err(MemoryError::BorrowConflict);
        }
        if self.borrows.len() >= self.max_tokens {
            return Err(MemoryError::ResourceLimit);
        }
        let token = self.next_token;
        self.next_token = self
            .next_token
            .checked_add(1)
            .ok_or(MemoryError::Overflow)?;
        self.allocations[handle.slot as usize].write_borrow = true;
        self.borrows.insert(
            token,
            BorrowRecord {
                slot: handle.slot,
                generation: handle.allocation_generation,
                kind: BorrowKind::Write,
            },
        );
        Ok(token)
    }

    pub fn end_borrow(&mut self, token: u64) -> Result<(), MemoryError> {
        let record = self
            .borrows
            .remove(&token)
            .ok_or(MemoryError::InvalidBorrow)?;
        let allocation = self
            .allocations
            .get_mut(record.slot as usize)
            .ok_or(MemoryError::InvalidBorrow)?;
        if allocation.generation != record.generation || !allocation.live {
            return Err(MemoryError::InvalidBorrow);
        }
        match record.kind {
            BorrowKind::Read => allocation.read_borrows -= 1,
            BorrowKind::Write => allocation.write_borrow = false,
        }
        Ok(())
    }

    pub fn free(&mut self, owner: u64, handle: Handle) -> Result<(), MemoryError> {
        self.require_owner(owner)?;
        let allocation = self.allocation(handle).map_err(|error| {
            if error == MemoryError::UseAfterFree {
                MemoryError::DoubleFree
            } else {
                error
            }
        })?;
        if handle.offset != 0 || handle.length != allocation.length {
            return Err(MemoryError::NotRoot);
        }
        if allocation.read_borrows > 0 || allocation.write_borrow {
            return Err(MemoryError::BorrowConflict);
        }
        let bytes = allocation.data.len() as u64;
        let next_generation = allocation
            .generation
            .checked_add(1)
            .ok_or(MemoryError::Overflow)?;
        let slot = handle.slot as usize;
        self.allocations[slot].live = false;
        self.allocations[slot].data.clear();
        self.allocations[slot].generation = next_generation;
        self.used_bytes -= bytes;
        Ok(())
    }

    pub fn transfer(&mut self, old: u64, new: u64) -> Result<(), MemoryError> {
        self.require_owner(old)?;
        if new == 0 {
            return Err(MemoryError::InvalidOwner);
        }
        if self.released {
            return Err(MemoryError::ArenaReleased);
        }
        if !self.borrows.is_empty() {
            return Err(MemoryError::BorrowConflict);
        }
        self.owner = new;
        Ok(())
    }

    pub fn release(&mut self, owner: u64) -> Result<(), MemoryError> {
        self.require_owner(owner)?;
        if self.released {
            return Err(MemoryError::ArenaReleased);
        }
        if !self.borrows.is_empty() {
            return Err(MemoryError::BorrowConflict);
        }
        let next_epoch = self.epoch.checked_add(1).ok_or(MemoryError::Overflow)?;
        let next_generations = self
            .allocations
            .iter()
            .map(|allocation| {
                allocation
                    .generation
                    .checked_add(1)
                    .ok_or(MemoryError::Overflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.released = true;
        self.epoch = next_epoch;
        self.used_bytes = 0;
        for (allocation, generation) in self.allocations.iter_mut().zip(next_generations) {
            allocation.live = false;
            allocation.data.clear();
            allocation.generation = generation;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaBuffer {
    handle: Option<Handle>,
    len: u64,
    capacity: u64,
}

impl ArenaBuffer {
    pub fn new() -> Self {
        Self {
            handle: None,
            len: 0,
            capacity: 0,
        }
    }

    pub fn push(&mut self, arena: &mut Arena, owner: u64, value: u8) -> Result<(), MemoryError> {
        let before_arena = arena.clone();
        let before_self = self.clone();
        let result = (|| {
            if self.len == self.capacity {
                let next = if self.capacity == 0 {
                    4
                } else {
                    self.capacity.checked_mul(2).ok_or(MemoryError::Overflow)?
                };
                let next_handle = arena.alloc(owner, TYPE_BYTE, next, 1, true)?;
                if let Some(old) = self.handle {
                    for index in 0..self.len {
                        let byte = arena.read_element(old, index)?;
                        arena.write_element(next_handle, index, &byte)?;
                    }
                    arena.free(owner, old)?;
                }
                self.handle = Some(next_handle);
                self.capacity = next;
            }
            arena.write_element(
                self.handle.ok_or(MemoryError::InvalidHandle)?,
                self.len,
                &[value],
            )?;
            self.len += 1;
            Ok(())
        })();
        if let Err(error) = result {
            *arena = before_arena;
            *self = before_self;
            return Err(error);
        }
        Ok(())
    }

    pub fn values(&self, arena: &Arena) -> Result<Vec<u8>, MemoryError> {
        let mut values = Vec::new();
        if let Some(handle) = self.handle {
            for index in 0..self.len {
                values.push(arena.read_element(handle, index)?[0]);
            }
        }
        Ok(values)
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

impl Default for ArenaBuffer {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_node(value: i64, left: Option<Handle>, right: Option<Handle>) -> [u8; NODE_SIZE] {
    let mut bytes = [0_u8; NODE_SIZE];
    bytes[0..8].copy_from_slice(&value.to_le_bytes());
    if let Some(handle) = left {
        bytes[8] = 1;
        bytes[16..64].copy_from_slice(&handle.encode());
    }
    if let Some(handle) = right {
        bytes[64] = 1;
        bytes[72..120].copy_from_slice(&handle.encode());
    }
    bytes
}

fn decode_node(bytes: &[u8]) -> Result<(i64, Option<Handle>, Option<Handle>), MemoryError> {
    if bytes.len() != NODE_SIZE || !matches!(bytes[8], 0 | 1) || !matches!(bytes[64], 0 | 1) {
        return Err(MemoryError::TypeMismatch);
    }
    let value = i64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let left = if bytes[8] == 1 {
        Some(Handle::decode(&bytes[16..64])?)
    } else {
        None
    };
    let right = if bytes[64] == 1 {
        Some(Handle::decode(&bytes[72..120])?)
    } else {
        None
    };
    Ok((value, left, right))
}

pub fn build_complete_tree(
    arena: &mut Arena,
    owner: u64,
    depth: u32,
    max_depth: u32,
) -> Result<Handle, MemoryError> {
    if depth == 0 || depth > max_depth || depth >= 63 {
        return Err(MemoryError::DepthLimit);
    }
    let nodes = (1_u64.checked_shl(depth).ok_or(MemoryError::Overflow)?) - 1;
    let required = nodes
        .checked_mul(NODE_SIZE as u64)
        .ok_or(MemoryError::Overflow)?;
    if arena
        .used_bytes
        .checked_add(required)
        .ok_or(MemoryError::Overflow)?
        > arena.byte_limit
    {
        return Err(MemoryError::OutOfMemory);
    }
    let before = arena.clone();
    fn build(arena: &mut Arena, owner: u64, depth: u32, value: i64) -> Result<Handle, MemoryError> {
        let left = if depth > 1 {
            Some(build(arena, owner, depth - 1, value * 2)?)
        } else {
            None
        };
        let right = if depth > 1 {
            Some(build(arena, owner, depth - 1, value * 2 + 1)?)
        } else {
            None
        };
        let handle = arena.alloc(owner, TYPE_NODE, 1, NODE_SIZE as u64, true)?;
        arena.write_element(handle, 0, &encode_node(value, left, right))?;
        Ok(handle)
    }
    match build(arena, owner, depth, 1) {
        Ok(root) => Ok(root),
        Err(error) => {
            *arena = before;
            Err(error)
        }
    }
}

pub fn tree_sum(arena: &Arena, root: Handle, max_depth: u32) -> Result<i64, MemoryError> {
    fn visit(
        arena: &Arena,
        handle: Handle,
        depth: u32,
        max_depth: u32,
    ) -> Result<i64, MemoryError> {
        if depth > max_depth {
            return Err(MemoryError::DepthLimit);
        }
        let (value, left, right) = decode_node(&arena.read_element(handle, 0)?)?;
        let mut total = value;
        if let Some(child) = left {
            total = total
                .checked_add(visit(arena, child, depth + 1, max_depth)?)
                .ok_or(MemoryError::Overflow)?;
        }
        if let Some(child) = right {
            total = total
                .checked_add(visit(arena, child, depth + 1, max_depth)?)
                .ok_or(MemoryError::Overflow)?;
        }
        Ok(total)
    }
    visit(arena, root, 1, max_depth)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HostProfile {
    Compiler,
    Agent,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HostOperation {
    PackageRead,
    BuildWrite,
    ClockMetadata,
    LinkerInvoke,
    FsRead,
    FsWrite,
    ClockRead,
    ProcessSpawn,
    ArtifactRead,
    AnchorRead,
    ArbitraryFfi,
    ShellEval,
}

fn profile_allows(profile: HostProfile, operation: HostOperation) -> bool {
    matches!(
        (profile, operation),
        (HostProfile::Compiler, HostOperation::PackageRead)
            | (HostProfile::Compiler, HostOperation::BuildWrite)
            | (HostProfile::Compiler, HostOperation::ClockMetadata)
            | (HostProfile::Compiler, HostOperation::LinkerInvoke)
            | (HostProfile::Agent, HostOperation::FsRead)
            | (HostProfile::Agent, HostOperation::FsWrite)
            | (HostProfile::Agent, HostOperation::ClockRead)
            | (HostProfile::Agent, HostOperation::ProcessSpawn)
            | (HostProfile::Verification, HostOperation::ArtifactRead)
            | (HostProfile::Verification, HostOperation::AnchorRead)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub profile: HostProfile,
    pub operation: HostOperation,
    pub budget: u64,
    pub expires_at: u64,
}

#[derive(Debug, Default)]
pub struct HostGate {
    pub dispatched: u64,
}

impl HostGate {
    pub fn authorize(
        &mut self,
        arena: &Arena,
        input: Handle,
        profile: HostProfile,
        operation: HostOperation,
        capability: Option<&mut Capability>,
        now: u64,
    ) -> Result<(), MemoryError> {
        arena.read_element(input, 0)?;
        if !profile_allows(profile, operation) {
            return Err(MemoryError::OperationDenied);
        }
        let capability = capability.ok_or(MemoryError::CapabilityMissing)?;
        if capability.profile != profile || capability.operation != operation {
            return Err(MemoryError::ProfileMismatch);
        }
        if capability.expires_at <= now || capability.budget == 0 {
            return Err(MemoryError::OperationDenied);
        }
        capability.budget -= 1;
        self.dispatched += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AbiField {
    pub name: String,
    pub offset: usize,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub required: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AbiPolicy {
    pub schema_version: u32,
    pub abi_version: u32,
    pub endianness: String,
    pub handle_size: usize,
    pub result_size: usize,
    pub host_call_size: usize,
    pub permissions: BTreeMap<String, u32>,
    pub handle_fields: Vec<AbiField>,
    pub result_fields: Vec<AbiField>,
    pub host_call_fields: Vec<AbiField>,
    pub errors: BTreeMap<String, u32>,
    pub profiles: BTreeMap<String, Vec<String>>,
    pub forbidden_operations: Vec<String>,
}

pub fn validate_abi_policy(policy: &AbiPolicy) -> Vec<String> {
    let mut failures = Vec::new();
    if policy.schema_version != 1
        || policy.abi_version != 1
        || policy.endianness != "little"
        || policy.handle_size != HANDLE_SIZE
        || policy.result_size != 56
        || policy.host_call_size != 120
    {
        failures.push("ABI version, byte order or descriptor size mismatch".into());
    }
    let expected_permissions =
        BTreeMap::from([("read".to_owned(), 1), ("read_write".to_owned(), 2)]);
    if policy.permissions != expected_permissions {
        failures.push("permission codes mismatch".into());
    }
    let fields = |items: &[AbiField]| {
        items
            .iter()
            .map(|field| (field.name.clone(), field.offset, field.kind.clone()))
            .collect::<Vec<_>>()
    };
    let expected_handle = vec![
        ("arena_id".into(), 0, "u64".into()),
        ("arena_epoch".into(), 8, "u64".into()),
        ("slot".into(), 16, "u32".into()),
        ("allocation_generation".into(), 20, "u32".into()),
        ("offset".into(), 24, "u64".into()),
        ("length".into(), 32, "u64".into()),
        ("type_id".into(), 40, "u32".into()),
        ("permission".into(), 44, "u8".into()),
        ("reserved".into(), 45, "bytes[3]".into()),
    ];
    if fields(&policy.handle_fields) != expected_handle
        || policy
            .handle_fields
            .last()
            .and_then(|field| field.required.as_deref())
            != Some("zero")
    {
        failures.push("CoreHandleV1 field contract mismatch".into());
    }
    let expected_result = vec![
        ("abi_version".into(), 0, "u16".into()),
        ("status".into(), 2, "u16".into()),
        ("error_code".into(), 4, "u32".into()),
        ("payload".into(), 8, "CoreHandleV1".into()),
    ];
    if fields(&policy.result_fields) != expected_result {
        failures.push("CoreResultV1 field contract mismatch".into());
    }
    let expected_call = vec![
        ("abi_version".into(), 0, "u16".into()),
        ("profile".into(), 2, "u16".into()),
        ("operation".into(), 4, "u32".into()),
        ("request_id".into(), 8, "u64".into()),
        ("capability".into(), 16, "CoreHandleV1".into()),
        ("input".into(), 64, "CoreHandleV1".into()),
        ("deadline_monotonic".into(), 112, "u64".into()),
    ];
    if fields(&policy.host_call_fields) != expected_call {
        failures.push("HostCallV1 field contract mismatch".into());
    }
    let expected_errors = BTreeMap::from([
        ("InvalidHandle".into(), 1),
        ("UseAfterFree".into(), 2),
        ("ArenaReleased".into(), 3),
        ("OutOfBounds".into(), 4),
        ("PermissionDenied".into(), 5),
        ("BorrowConflict".into(), 6),
        ("InvalidBorrow".into(), 7),
        ("NotOwner".into(), 8),
        ("InvalidOwner".into(), 9),
        ("NotRoot".into(), 10),
        ("DoubleFree".into(), 11),
        ("Overflow".into(), 12),
        ("OutOfMemory".into(), 13),
        ("ResourceLimit".into(), 14),
        ("TypeMismatch".into(), 15),
        ("DepthLimit".into(), 16),
        ("CapabilityMissing".into(), 32),
        ("ProfileMismatch".into(), 33),
        ("OperationDenied".into(), 34),
        ("VersionUnsupported".into(), 35),
        ("Utf8Invalid".into(), 36),
        ("HostUnavailable".into(), 37),
    ]);
    if policy.errors != expected_errors {
        failures.push("stable error code table mismatch".into());
    }
    let expected_profiles = BTreeMap::from([
        (
            "agent-runtime".into(),
            vec![
                "fs.read".into(),
                "fs.write".into(),
                "clock.read".into(),
                "process.spawn".into(),
            ],
        ),
        (
            "compiler-host".into(),
            vec![
                "package.read".into(),
                "build.write".into(),
                "clock.metadata".into(),
                "linker.invoke".into(),
            ],
        ),
        (
            "verification-host".into(),
            vec!["artifact.read".into(), "anchor.read".into()],
        ),
    ]);
    if policy.profiles != expected_profiles {
        failures.push("host profile operation table mismatch".into());
    }
    if policy.forbidden_operations
        != [
            "ffi.call",
            "shell.eval",
            "pointer.import",
            "environment.all",
        ]
    {
        failures.push("forbidden operation table mismatch".into());
    }
    failures
}
