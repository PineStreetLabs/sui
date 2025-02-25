// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    diagnostics::WarningFilters,
    parser::ast::{
        self as P, Ability, Ability_, BinOp, ConstantName, Field, FunctionName, ModuleName,
        Mutability, QuantKind, SpecApplyPattern, StructName, UnaryOp, Var, ENTRY_MODIFIER,
    },
    shared::{
        ast_debug::*, known_attributes::KnownAttribute, unique_map::UniqueMap,
        unique_set::UniqueSet, *,
    },
};
use move_ir_types::location::*;
use move_symbol_pool::Symbol;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    hash::Hash,
};

//**************************************************************************************************
// Program
//**************************************************************************************************

#[derive(Debug, Clone)]
pub struct Program {
    // Map of declared named addresses, and their values if specified
    pub modules: UniqueMap<ModuleIdent, ModuleDefinition>,
    pub scripts: BTreeMap<Symbol, Script>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitUseFun {
    pub loc: Loc,
    pub attributes: Attributes,
    pub is_public: Option<Loc>,
    pub function: ModuleAccess,
    pub ty: ModuleAccess,
    pub method: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplicitUseFunKind {
    // From a function declaration in the module
    FunctionDeclaration,
    // From a normal, non 'use fun' use declaration,
    UseAlias { used: bool },
}

// These are only candidates as we have not yet checked if they have the proper signature for a
// use fun declaration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplicitUseFunCandidate {
    pub loc: Loc,
    pub attributes: Attributes,
    pub is_public: Option<Loc>,
    pub function: (ModuleIdent, Name),
    pub kind: ImplicitUseFunKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseFuns {
    pub explicit: Vec<ExplicitUseFun>,
    pub implicit: UniqueMap<Name, ImplicitUseFunCandidate>,
}

//**************************************************************************************************
// Attributes
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeValue_ {
    Value(Value),
    Module(ModuleIdent),
    ModuleAccess(ModuleAccess),
}
pub type AttributeValue = Spanned<AttributeValue_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribute_ {
    Name(Name),
    Assigned(Name, Box<AttributeValue>),
    Parameterized(Name, Attributes),
}
pub type Attribute = Spanned<Attribute_>;

impl Attribute_ {
    pub fn attribute_name(&self) -> &Name {
        match self {
            Attribute_::Name(nm)
            | Attribute_::Assigned(nm, _)
            | Attribute_::Parameterized(nm, _) => nm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeName_ {
    Unknown(Symbol),
    Known(KnownAttribute),
}

impl AttributeName_ {
    pub fn name(&self) -> Symbol {
        match self {
            Self::Unknown(s) => *s,
            Self::Known(a) => a.name().into(),
        }
    }
}

pub type AttributeName = Spanned<AttributeName_>;

pub type Attributes = UniqueMap<AttributeName, Attribute>;

//**************************************************************************************************
// Scripts
//**************************************************************************************************

#[derive(Debug, Clone)]
pub struct Script {
    pub warning_filter: WarningFilters,
    // package name metadata from compiler arguments.
    // It is used primarily for retrieving the associated `PackageConfig`,
    // but it is also used in determining public(package) visibility.
    pub package_name: Option<Symbol>,
    pub attributes: Attributes,
    pub loc: Loc,
    pub use_funs: UseFuns,
    pub constants: UniqueMap<ConstantName, Constant>,
    pub function_name: FunctionName,
    pub function: Function,
    pub specs: Vec<SpecBlock>,
}

//**************************************************************************************************
// Modules
//**************************************************************************************************

#[derive(Clone, Copy)]
pub enum Address {
    Numerical {
        name: Option<Name>,
        value: Spanned<NumericalAddress>,
        // set to true when the same name is used across multiple packages
        name_conflict: bool,
    },
    NamedUnassigned(Name),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleIdent_ {
    pub address: Address,
    pub module: ModuleName,
}
pub type ModuleIdent = Spanned<ModuleIdent_>;

#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    pub warning_filter: WarningFilters,
    // package name metadata from compiler arguments, not used for any language rules
    pub package_name: Option<Symbol>,
    pub attributes: Attributes,
    pub loc: Loc,
    pub is_source_module: bool,
    pub use_funs: UseFuns,
    pub friends: UniqueMap<ModuleIdent, Friend>,
    pub structs: UniqueMap<StructName, StructDefinition>,
    pub functions: UniqueMap<FunctionName, Function>,
    pub constants: UniqueMap<ConstantName, Constant>,
    pub specs: Vec<SpecBlock>,
}

//**************************************************************************************************
// Friend
//**************************************************************************************************

#[derive(Debug, Clone)]
pub struct Friend {
    pub attributes: Attributes,
    pub loc: Loc,
}

//**************************************************************************************************
// Structs
//**************************************************************************************************

pub type Fields<T> = UniqueMap<Field, (usize, T)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructTypeParameter {
    pub is_phantom: bool,
    pub name: Name,
    pub constraints: AbilitySet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub warning_filter: WarningFilters,
    // index in the original order as defined in the source file
    pub index: usize,
    pub attributes: Attributes,
    pub loc: Loc,
    pub abilities: AbilitySet,
    pub type_parameters: Vec<StructTypeParameter>,
    pub fields: StructFields,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructFields {
    Positional(Vec<Type>),
    Named(Fields<Type>),
    Native(Loc),
}

//**************************************************************************************************
// Functions
//**************************************************************************************************

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Visibility {
    Public(Loc),
    Friend(Loc),
    Package(Loc),
    Internal,
}

#[derive(PartialEq, Clone, Debug)]
pub struct FunctionSignature {
    pub type_parameters: Vec<(Name, AbilitySet)>,
    pub parameters: Vec<(Mutability, Var, Type)>,
    pub return_type: Type,
}

#[derive(PartialEq, Clone, Debug)]
pub enum FunctionBody_ {
    Defined(Sequence),
    Native,
}
pub type FunctionBody = Spanned<FunctionBody_>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpecId(usize);

#[derive(PartialEq, Clone, Debug)]
pub struct Function {
    pub warning_filter: WarningFilters,
    // index in the original order as defined in the source file
    pub index: usize,
    pub attributes: Attributes,
    pub loc: Loc,
    pub visibility: Visibility,
    pub entry: Option<Loc>,
    pub signature: FunctionSignature,
    pub acquires: Vec<ModuleAccess>,
    pub body: FunctionBody,
    pub specs: BTreeMap<SpecId, SpecBlock>,
}

//**************************************************************************************************
// Constants
//**************************************************************************************************

#[derive(PartialEq, Clone, Debug)]
pub struct Constant {
    pub warning_filter: WarningFilters,
    // index in the original order as defined in the source file
    pub index: usize,
    pub attributes: Attributes,
    pub loc: Loc,
    pub signature: Type,
    pub value: Exp,
}

//**************************************************************************************************
// Specification Blocks
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBlock_ {
    pub attributes: Attributes,
    pub target: SpecBlockTarget,
    pub members: Vec<SpecBlockMember>,
}
pub type SpecBlock = Spanned<SpecBlock_>;

#[derive(Debug, Clone, PartialEq)]
pub enum SpecBlockTarget_ {
    Code,
    Module,
    Member(Name, Option<Box<FunctionSignature>>),
    Schema(Name, Vec<(Name, AbilitySet)>),
}

pub type SpecBlockTarget = Spanned<SpecBlockTarget_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum SpecBlockMember_ {
    Condition {
        kind: SpecConditionKind,
        properties: Vec<PragmaProperty>,
        exp: Exp,
        additional_exps: Vec<Exp>,
    },
    Function {
        uninterpreted: bool,
        name: FunctionName,
        signature: FunctionSignature,
        body: FunctionBody,
    },
    Variable {
        is_global: bool,
        name: Name,
        type_parameters: Vec<(Name, AbilitySet)>,
        type_: Type,
        init: Option<Exp>,
    },
    Update {
        lhs: Exp,
        rhs: Exp,
    },
    Let {
        name: Name,
        post_state: bool,
        def: Exp,
    },
    Include {
        properties: Vec<PragmaProperty>,
        exp: Exp,
    },
    Apply {
        exp: Exp,
        patterns: Vec<SpecApplyPattern>,
        exclusion_patterns: Vec<SpecApplyPattern>,
    },
    Pragma {
        properties: Vec<PragmaProperty>,
    },
}
pub type SpecBlockMember = Spanned<SpecBlockMember_>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SpecConditionKind_ {
    Assert,
    Assume,
    Decreases,
    AbortsIf,
    AbortsWith,
    SucceedsIf,
    Modifies,
    Emits,
    Ensures,
    Requires,
    Invariant(Vec<(Name, AbilitySet)>),
    InvariantUpdate(Vec<(Name, AbilitySet)>),
    Axiom(Vec<(Name, AbilitySet)>),
}
pub type SpecConditionKind = Spanned<SpecConditionKind_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PragmaProperty_ {
    pub name: Name,
    pub value: Option<PragmaValue>,
}
pub type PragmaProperty = Spanned<PragmaProperty_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PragmaValue {
    Literal(Value),
    Ident(ModuleAccess),
}

//**************************************************************************************************
// Types
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbilitySet(UniqueSet<Ability>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ModuleAccess_ {
    Name(Name),
    ModuleAccess(ModuleIdent, Name),
}
pub type ModuleAccess = Spanned<ModuleAccess_>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Type_ {
    Unit,
    Multiple(Vec<Type>),
    Apply(ModuleAccess, Vec<Type>),
    Ref(bool, Box<Type>),
    Fun(Vec<Type>, Box<Type>),
    UnresolvedError,
}
pub type Type = Spanned<Type_>;

//**************************************************************************************************
// Expressions
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq)]
pub enum FieldBindings {
    Named(Fields<LValue>),
    Positional(Vec<LValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LValue_ {
    Var(Mutability, ModuleAccess, Option<Vec<Type>>),
    Unpack(ModuleAccess, Option<Vec<Type>>, FieldBindings),
}
pub type LValue = Spanned<LValue_>;
pub type LValueList_ = Vec<LValue>;
pub type LValueList = Spanned<LValueList_>;

pub type LValueWithRange_ = (LValue, Exp);
pub type LValueWithRange = Spanned<LValueWithRange_>;
pub type LValueWithRangeList_ = Vec<LValueWithRange>;
pub type LValueWithRangeList = Spanned<LValueWithRangeList_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ExpDotted_ {
    Exp(Box<Exp>),
    Dot(Box<ExpDotted>, Name),
}
pub type ExpDotted = Spanned<ExpDotted_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value_ {
    // 0x<hex representation up to 64 digits with padding 0s>
    Address(Address),
    // <num>
    InferredNum(move_core_types::u256::U256),
    // <num>u8
    U8(u8),
    // <num>u16
    U16(u16),
    // <num>u32
    U32(u32),
    // <num>u64
    U64(u64),
    // <num>u128
    U128(u128),
    // <num>u256
    U256(move_core_types::u256::U256),
    // true
    // false
    Bool(bool),
    Bytearray(Vec<u8>),
}
pub type Value = Spanned<Value_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Exp_ {
    Value(Value),
    Move(Var),
    Copy(Var),

    Name(ModuleAccess, Option<Vec<Type>>),
    Call(
        ModuleAccess,
        /* is_macro */ bool,
        Option<Vec<Type>>,
        Spanned<Vec<Exp>>,
    ),
    MethodCall(Box<ExpDotted>, Name, Option<Vec<Type>>, Spanned<Vec<Exp>>),
    Pack(ModuleAccess, Option<Vec<Type>>, Fields<Exp>),
    Vector(Loc, Option<Vec<Type>>, Spanned<Vec<Exp>>),

    IfElse(Box<Exp>, Box<Exp>, Box<Exp>),
    While(Box<Exp>, Box<Exp>),
    Loop(Box<Exp>),
    Block(Sequence),
    Lambda(LValueList, Box<Exp>), // spec only
    Quant(
        QuantKind,
        LValueWithRangeList,
        Vec<Vec<Exp>>,
        Option<Box<Exp>>,
        Box<Exp>,
    ), // spec only

    Assign(LValueList, Box<Exp>),
    FieldMutate(Box<ExpDotted>, Box<Exp>),
    Mutate(Box<Exp>, Box<Exp>),

    Return(Box<Exp>),
    Abort(Box<Exp>),
    Break,
    Continue,

    Dereference(Box<Exp>),
    UnaryExp(UnaryOp, Box<Exp>),
    BinopExp(Box<Exp>, BinOp, Box<Exp>),

    ExpList(Vec<Exp>),
    Unit {
        trailing: bool,
    },

    Borrow(bool, Box<Exp>),
    ExpDotted(Box<ExpDotted>),
    Index(Box<Exp>, Box<Exp>), // spec only (no mutation needed right now)

    Cast(Box<Exp>, Type),
    Annotate(Box<Exp>, Type),

    Spec(SpecId, BTreeSet<Name>),

    UnresolvedError,
}
pub type Exp = Spanned<Exp_>;

pub type Sequence = (UseFuns, VecDeque<SequenceItem>);
#[derive(Debug, Clone, PartialEq)]
pub enum SequenceItem_ {
    Seq(Exp),
    Declare(LValueList, Option<Type>),
    Bind(LValueList, Exp),
}
pub type SequenceItem = Spanned<SequenceItem_>;

//**************************************************************************************************
// Traits
//**************************************************************************************************

impl TName for ModuleIdent {
    type Key = ModuleIdent_;
    type Loc = Loc;

    fn drop_loc(self) -> (Loc, ModuleIdent_) {
        (self.loc, self.value)
    }

    fn add_loc(loc: Loc, value: ModuleIdent_) -> ModuleIdent {
        sp(loc, value)
    }

    fn borrow(&self) -> (&Loc, &ModuleIdent_) {
        (&self.loc, &self.value)
    }
}

impl TName for AttributeName {
    type Key = AttributeName_;
    type Loc = Loc;

    fn drop_loc(self) -> (Self::Loc, Self::Key) {
        let sp!(loc, n_) = self;
        (loc, n_)
    }

    fn add_loc(loc: Self::Loc, name_: Self::Key) -> Self {
        sp(loc, name_)
    }

    fn borrow(&self) -> (&Self::Loc, &Self::Key) {
        let sp!(loc, n_) = self;
        (loc, n_)
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Numerical { value: l, .. }, Self::Numerical { value: r, .. }) => l == r,
            (Self::NamedUnassigned(l), Self::NamedUnassigned(r)) => l == r,
            _ => false,
        }
    }
}

impl Eq for Address {}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            (Self::Numerical { .. }, Self::NamedUnassigned(_)) => Ordering::Less,
            (Self::NamedUnassigned(_), Self::Numerical { .. }) => Ordering::Greater,

            (Self::Numerical { value: l, .. }, Self::Numerical { value: r, .. }) => l.cmp(r),
            (Self::NamedUnassigned(l), Self::NamedUnassigned(r)) => l.cmp(r),
        }
    }
}

impl Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Numerical {
                value: sp!(_, bytes),
                ..
            } => bytes.hash(state),
            Self::NamedUnassigned(name) => name.hash(state),
        }
    }
}

//**************************************************************************************************
// impls
//**************************************************************************************************

impl UseFuns {
    pub fn new() -> Self {
        Self {
            explicit: vec![],
            implicit: UniqueMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        let Self { explicit, implicit } = self;
        explicit.is_empty() && implicit.is_empty()
    }
}

impl Address {
    pub const fn anonymous(loc: Loc, address: NumericalAddress) -> Self {
        Self::Numerical {
            name: None,
            value: sp(loc, address),
            name_conflict: false,
        }
    }

    pub fn into_addr_bytes(self) -> NumericalAddress {
        match self {
            Self::Numerical {
                value: sp!(_, bytes),
                ..
            } => bytes,
            Self::NamedUnassigned(_) => NumericalAddress::DEFAULT_ERROR_ADDRESS,
        }
    }

    pub fn is(&self, address: impl AsRef<str>) -> bool {
        match self {
            Self::Numerical { name: Some(n), .. } | Self::NamedUnassigned(n) => {
                n.value.as_str() == address.as_ref()
            }
            Self::Numerical { name: None, .. } => false,
        }
    }
}

impl ModuleIdent_ {
    pub fn new(address: Address, module: ModuleName) -> Self {
        Self { address, module }
    }

    pub fn is(&self, address: impl AsRef<str>, module: impl AsRef<str>) -> bool {
        let Self {
            address: a,
            module: m,
        } = self;
        a.is(address) && m == module.as_ref()
    }
}

impl SpecId {
    pub fn new(u: usize) -> Self {
        SpecId(u)
    }

    pub fn inner(self) -> usize {
        self.0
    }
}

impl AbilitySet {
    /// All abilities
    pub const ALL: [Ability_; 4] = [
        Ability_::Copy,
        Ability_::Drop,
        Ability_::Store,
        Ability_::Key,
    ];
    /// Abilities for bool, u8, u16, u32, u64, u128, u256 and address
    pub const PRIMITIVES: [Ability_; 3] = [Ability_::Copy, Ability_::Drop, Ability_::Store];
    /// Abilities for &_ and &mut _
    pub const REFERENCES: [Ability_; 2] = [Ability_::Copy, Ability_::Drop];
    /// Abilities for signer
    pub const SIGNER: [Ability_; 1] = [Ability_::Drop];
    /// Abilities for vector<_>, note they are predicated on the type argument
    pub const COLLECTION: [Ability_; 3] = [Ability_::Copy, Ability_::Drop, Ability_::Store];

    pub fn empty() -> Self {
        AbilitySet(UniqueSet::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn add(&mut self, a: Ability) -> Result<(), Loc> {
        self.0.add(a).map_err(|(_a, loc)| loc)
    }

    pub fn has_ability(&self, a: &Ability) -> bool {
        self.0.contains(a)
    }

    pub fn has_ability_(&self, a: Ability_) -> bool {
        self.0.contains_(&a)
    }

    pub fn ability_loc(&self, sp!(_, a_): &Ability) -> Option<Loc> {
        self.0.get_loc_(a_).copied()
    }

    pub fn ability_loc_(&self, a: Ability_) -> Option<Loc> {
        self.0.get_loc_(&a).copied()
    }

    // intersection of two sets. Keeps the loc of the first set
    pub fn intersect(&self, other: &Self) -> Self {
        Self(self.0.intersect(&other.0))
    }

    // union of two sets. Prefers the loc of the first set
    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0))
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    pub fn iter(&self) -> AbilitySetIter {
        self.into_iter()
    }

    pub fn from_abilities(
        iter: impl IntoIterator<Item = Ability>,
    ) -> Result<Self, (Ability_, Loc, Loc)> {
        Ok(Self(UniqueSet::from_elements(iter)?))
    }

    pub fn from_abilities_(
        loc: Loc,
        iter: impl IntoIterator<Item = Ability_>,
    ) -> Result<Self, (Ability_, Loc, Loc)> {
        Ok(Self(UniqueSet::from_elements_(loc, iter)?))
    }

    pub fn all(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::ALL.to_vec()).unwrap()
    }

    pub fn primitives(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::PRIMITIVES.to_vec()).unwrap()
    }

    pub fn references(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::REFERENCES.to_vec()).unwrap()
    }

    pub fn signer(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::SIGNER.to_vec()).unwrap()
    }

    pub fn collection(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::COLLECTION.to_vec()).unwrap()
    }
}

impl Visibility {
    pub const FRIEND: &'static str = P::Visibility::FRIEND;
    pub const FRIEND_IDENT: &'static str = P::Visibility::FRIEND_IDENT;
    pub const INTERNAL: &'static str = P::Visibility::INTERNAL;
    pub const PACKAGE: &'static str = P::Visibility::PACKAGE;
    pub const PACKAGE_IDENT: &'static str = P::Visibility::PACKAGE_IDENT;
    pub const PUBLIC: &'static str = P::Visibility::PUBLIC;

    pub fn loc(&self) -> Option<Loc> {
        match self {
            Visibility::Friend(loc) | Visibility::Package(loc) | Visibility::Public(loc) => {
                Some(*loc)
            }
            Visibility::Internal => None,
        }
    }
}

//**************************************************************************************************
// Iter
//**************************************************************************************************

pub struct AbilitySetIter<'a>(unique_set::Iter<'a, Ability>);

impl<'a> Iterator for AbilitySetIter<'a> {
    type Item = Ability;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(loc, a_)| sp(loc, *a_))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> IntoIterator for &'a AbilitySet {
    type Item = Ability;
    type IntoIter = AbilitySetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AbilitySetIter(self.0.iter())
    }
}

pub struct AbilitySetIntoIter(unique_set::IntoIter<Ability>);

impl Iterator for AbilitySetIntoIter {
    type Item = Ability;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl IntoIterator for AbilitySet {
    type Item = Ability;
    type IntoIter = AbilitySetIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        AbilitySetIntoIter(self.0.into_iter())
    }
}

//**************************************************************************************************
// Display
//**************************************************************************************************

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Numerical {
                name: None,
                value: sp!(_, bytes),
                ..
            } => write!(f, "{}", bytes),
            Self::Numerical {
                name: Some(name),
                value: sp!(_, bytes),
                name_conflict: true,
            } => write!(f, "({}={})", name, bytes),
            Self::Numerical {
                name: Some(name),
                value: _,
                name_conflict: false,
            }
            | Self::NamedUnassigned(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for AttributeName_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            AttributeName_::Unknown(sym) => write!(f, "{}", sym),
            AttributeName_::Known(known) => write!(f, "{}", known.name()),
        }
    }
}

impl fmt::Display for ModuleIdent_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}::{}", self.address, &self.module)
    }
}

impl fmt::Display for ModuleAccess_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        use ModuleAccess_::*;
        match self {
            Name(n) => write!(f, "{}", n),
            ModuleAccess(m, n) => write!(f, "{}::{}", m, n),
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Visibility::Public(_) => Visibility::PUBLIC,
                Visibility::Friend(_) => Visibility::FRIEND,
                Visibility::Package(_) => Visibility::PACKAGE,
                Visibility::Internal => Visibility::INTERNAL,
            }
        )
    }
}

impl fmt::Display for Type_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        use Type_::*;
        match self {
            UnresolvedError => write!(f, "_"),
            Apply(n, tys) => {
                write!(f, "{}", n)?;
                if !tys.is_empty() {
                    write!(f, "<")?;
                    write!(f, "{}", format_comma(tys))?;
                    write!(f, ">")?;
                }
                Ok(())
            }
            Ref(mut_, ty) => write!(f, "&{}{}", if *mut_ { "mut " } else { "" }, ty),
            Fun(args, result) => write!(f, "({}):{}", format_comma(args), result),
            Unit => write!(f, "()"),
            Multiple(tys) => {
                write!(f, "(")?;
                write!(f, "{}", format_comma(tys))?;
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for SpecId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

//**************************************************************************************************
// Debug
//**************************************************************************************************

impl AstDebug for Program {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Program { modules, scripts } = self;
        for (m, mdef) in modules.key_cloned_iter() {
            w.write(&format!("module {}", m));
            w.block(|w| mdef.ast_debug(w));
            w.new_line();
        }

        for (n, s) in scripts {
            w.write(&format!("script {}", n));
            w.block(|w| s.ast_debug(w));
            w.new_line()
        }
    }
}

impl AstDebug for ExplicitUseFun {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Self {
            loc: _,
            attributes,
            is_public,
            function,
            ty,
            method,
        } = self;
        attributes.ast_debug(w);
        w.new_line();
        if is_public.is_some() {
            w.write("public ");
        }
        w.write("use fun ");
        function.ast_debug(w);
        w.write(" as ");
        ty.ast_debug(w);
        w.writeln(&format!(".{method};"));
    }
}

impl AstDebug for ImplicitUseFunCandidate {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Self {
            loc: _,
            attributes,
            is_public,
            function: (m, n),
            kind,
        } = self;
        attributes.ast_debug(w);
        w.new_line();
        if is_public.is_some() {
            w.write("public ");
        }
        let kind_str = match kind {
            ImplicitUseFunKind::UseAlias { used: true } => "#used",
            ImplicitUseFunKind::UseAlias { used: false } => "#unused",
            ImplicitUseFunKind::FunctionDeclaration => "#fundecl",
        };
        w.writeln(&format!("implcit{kind_str}#use fun {m}::{n};"));
    }
}

impl AstDebug for UseFuns {
    fn ast_debug(&self, w: &mut AstWriter) {
        let UseFuns {
            explicit: explict,
            implicit,
        } = self;
        for use_fun in explict {
            use_fun.ast_debug(w);
        }
        for (_, _, use_fun) in implicit {
            use_fun.ast_debug(w);
        }
    }
}

impl AstDebug for AttributeValue_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            AttributeValue_::Value(v) => v.ast_debug(w),
            AttributeValue_::Module(m) => w.write(&format!("{}", m)),
            AttributeValue_::ModuleAccess(n) => n.ast_debug(w),
        }
    }
}

impl AstDebug for Attribute_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            Attribute_::Name(n) => w.write(&format!("{}", n)),
            Attribute_::Assigned(n, v) => {
                w.write(&format!("{}", n));
                w.write(" = ");
                v.ast_debug(w);
            }
            Attribute_::Parameterized(n, inners) => {
                w.write(&format!("{}", n));
                w.write("(");
                w.list(inners, ", ", |w, (_, _, inner)| {
                    inner.ast_debug(w);
                    false
                });
                w.write(")");
            }
        }
    }
}

impl AstDebug for Attributes {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write("#[");
        w.list(self, ", ", |w, (_, _, attr)| {
            attr.ast_debug(w);
            false
        });
        w.write("]");
    }
}

impl AstDebug for Script {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Script {
            package_name,
            attributes,
            loc: _loc,
            use_funs,
            constants,
            function_name,
            function,
            specs,
            warning_filter,
        } = self;
        warning_filter.ast_debug(w);
        if let Some(n) = package_name {
            w.writeln(&format!("{}", n))
        }
        attributes.ast_debug(w);
        use_funs.ast_debug(w);
        for cdef in constants.key_cloned_iter() {
            cdef.ast_debug(w);
            w.new_line();
        }
        (*function_name, function).ast_debug(w);
        for spec in specs {
            spec.ast_debug(w);
            w.new_line();
        }
    }
}

impl AstDebug for ModuleDefinition {
    fn ast_debug(&self, w: &mut AstWriter) {
        let ModuleDefinition {
            package_name,
            attributes,
            loc: _loc,
            is_source_module,
            use_funs,
            friends,
            structs,
            functions,
            constants,
            specs,
            warning_filter,
        } = self;
        warning_filter.ast_debug(w);
        if let Some(n) = package_name {
            w.writeln(&format!("{}", n))
        }
        attributes.ast_debug(w);
        w.writeln(if *is_source_module {
            "source module"
        } else {
            "library module"
        });
        use_funs.ast_debug(w);
        for (mident, _loc) in friends.key_cloned_iter() {
            w.write(&format!("friend {};", mident));
            w.new_line();
        }
        for sdef in structs.key_cloned_iter() {
            sdef.ast_debug(w);
            w.new_line();
        }
        for cdef in constants.key_cloned_iter() {
            cdef.ast_debug(w);
            w.new_line();
        }
        for fdef in functions.key_cloned_iter() {
            fdef.ast_debug(w);
            w.new_line();
        }
        for spec in specs {
            spec.ast_debug(w);
            w.new_line();
        }
    }
}

pub fn ability_modifiers_ast_debug(w: &mut AstWriter, abilities: &AbilitySet) {
    if !abilities.is_empty() {
        w.write(" has ");
        w.list(abilities, " ", |w, ab| {
            ab.ast_debug(w);
            false
        });
    }
}

impl AstDebug for (StructName, &StructDefinition) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (
            name,
            StructDefinition {
                index,
                attributes,
                loc: _loc,
                abilities,
                type_parameters,
                fields,
                warning_filter,
            },
        ) = self;
        warning_filter.ast_debug(w);
        attributes.ast_debug(w);
        if let StructFields::Native(_) = fields {
            w.write("native ");
        }

        w.write(&format!("struct#{index} {name}"));
        type_parameters.ast_debug(w);
        ability_modifiers_ast_debug(w, abilities);
        match fields {
            StructFields::Named(fields) => w.block(|w| {
                w.list(fields, ",", |w, (_, f, idx_st)| {
                    let (idx, st) = idx_st;
                    w.write(&format!("{}#{}: ", idx, f));
                    st.ast_debug(w);
                    true
                });
            }),
            StructFields::Positional(fields) => w.block(|w| {
                w.list(fields.iter().enumerate(), ",", |w, (idx, ty)| {
                    w.write(&format!("{idx}#pos{idx}: "));
                    ty.ast_debug(w);
                    true
                });
            }),
            StructFields::Native(_) => (),
        }
    }
}

impl AstDebug for SpecBlock_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(" spec ");
        self.target.ast_debug(w);
        w.write("{");
        w.semicolon(&self.members, |w, m| m.ast_debug(w));
        w.write("}");
    }
}

impl AstDebug for SpecBlockTarget_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            SpecBlockTarget_::Code => {}
            SpecBlockTarget_::Module => w.write("module "),
            SpecBlockTarget_::Member(name, sign_opt) => {
                w.write(name.value);
                if let Some(sign) = sign_opt {
                    sign.ast_debug(w);
                }
            }
            SpecBlockTarget_::Schema(n, tys) => {
                w.write(&format!("schema {}", n.value));
                if !tys.is_empty() {
                    w.write("<");
                    w.list(tys, ", ", |w, ty| {
                        ty.ast_debug(w);
                        true
                    });
                    w.write(">");
                }
            }
        }
    }
}

impl AstDebug for SpecConditionKind_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use SpecConditionKind_::*;
        match self {
            Assert => w.write("assert "),
            Assume => w.write("assume "),
            Decreases => w.write("decreases "),
            AbortsIf => w.write("aborts_if "),
            AbortsWith => w.write("aborts_with "),
            SucceedsIf => w.write("succeeds_if "),
            Modifies => w.write("modifies "),
            Emits => w.write("emits "),
            Ensures => w.write("ensures "),
            Requires => w.write("requires "),
            Invariant(ty_params) => {
                w.write("invariant");
                ty_params.ast_debug(w);
                w.write(" ")
            }
            InvariantUpdate(ty_params) => {
                w.write("invariant");
                ty_params.ast_debug(w);
                w.write(" update ")
            }
            Axiom(ty_params) => {
                w.write("axiom");
                ty_params.ast_debug(w);
                w.write(" ")
            }
        }
    }
}

impl AstDebug for SpecBlockMember_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            SpecBlockMember_::Condition {
                kind,
                properties: _,
                exp,
                additional_exps,
            } => {
                kind.ast_debug(w);
                exp.ast_debug(w);
                w.list(additional_exps, ",", |w, e| {
                    e.ast_debug(w);
                    true
                });
            }
            SpecBlockMember_::Function {
                uninterpreted,
                signature,
                name,
                body,
            } => {
                if *uninterpreted {
                    w.write("uninterpreted ")
                } else if let FunctionBody_::Native = &body.value {
                    w.write("native ");
                }
                w.write(&format!("define {}", name));
                signature.ast_debug(w);
                match &body.value {
                    FunctionBody_::Defined(body) => body.ast_debug(w),
                    FunctionBody_::Native => w.writeln(";"),
                }
            }
            SpecBlockMember_::Variable {
                is_global,
                name,
                type_parameters,
                type_,
                init: _,
            } => {
                if *is_global {
                    w.write("global ");
                } else {
                    w.write("local");
                }
                w.write(&format!("{}", name));
                type_parameters.ast_debug(w);
                w.write(": ");
                type_.ast_debug(w);
            }
            SpecBlockMember_::Update { lhs, rhs } => {
                w.write("update ");
                lhs.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            }
            SpecBlockMember_::Let {
                name,
                post_state,
                def,
            } => {
                w.write(&format!(
                    "let {}{} = ",
                    if *post_state { "post " } else { "" },
                    name
                ));
                def.ast_debug(w);
            }
            SpecBlockMember_::Include { properties: _, exp } => {
                w.write("include ");
                exp.ast_debug(w);
            }
            SpecBlockMember_::Apply {
                exp,
                patterns,
                exclusion_patterns,
            } => {
                w.write("apply ");
                exp.ast_debug(w);
                w.write(" to ");
                w.list(patterns, ", ", |w, p| {
                    p.ast_debug(w);
                    true
                });
                if !exclusion_patterns.is_empty() {
                    w.write(" exclude ");
                    w.list(exclusion_patterns, ", ", |w, p| {
                        p.ast_debug(w);
                        true
                    });
                }
            }
            SpecBlockMember_::Pragma { properties } => {
                w.write("pragma ");
                w.list(properties, ", ", |w, p| {
                    p.ast_debug(w);
                    true
                });
            }
        }
    }
}

impl AstDebug for PragmaProperty_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(self.name.value);
        if let Some(value) = &self.value {
            w.write(" = ");
            match value {
                PragmaValue::Literal(l) => l.ast_debug(w),
                PragmaValue::Ident(i) => i.ast_debug(w),
            }
        }
    }
}

impl AstDebug for (FunctionName, &Function) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (
            name,
            Function {
                index,
                attributes,
                loc: _loc,
                visibility,
                entry,
                signature,
                acquires,
                body,
                specs: _specs,
                warning_filter,
            },
        ) = self;
        warning_filter.ast_debug(w);
        attributes.ast_debug(w);
        visibility.ast_debug(w);
        if entry.is_some() {
            w.write(&format!("{} ", ENTRY_MODIFIER));
        }
        if let FunctionBody_::Native = &body.value {
            w.write("native ");
        }
        w.write(&format!("fun#{index} {name}"));
        signature.ast_debug(w);
        if !acquires.is_empty() {
            w.write(" acquires ");
            w.comma(acquires, |w, m| m.ast_debug(w));
            w.write(" ");
        }
        match &body.value {
            FunctionBody_::Defined(body) => body.ast_debug(w),
            FunctionBody_::Native => w.writeln(";"),
        }
    }
}

impl AstDebug for Visibility {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(&format!("{} ", self))
    }
}

impl AstDebug for FunctionSignature {
    fn ast_debug(&self, w: &mut AstWriter) {
        let FunctionSignature {
            type_parameters,
            parameters,
            return_type,
        } = self;
        type_parameters.ast_debug(w);
        w.write("(");
        w.comma(parameters, |w, (mutability, v, st)| {
            if mutability.is_some() {
                w.write("mut ");
            }
            w.write(&format!("{}: ", v));
            st.ast_debug(w);
        });
        w.write("): ");
        return_type.ast_debug(w)
    }
}

impl AstDebug for (ConstantName, &Constant) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (
            name,
            Constant {
                warning_filter,
                index,
                attributes,
                loc: _loc,
                signature,
                value,
            },
        ) = self;
        warning_filter.ast_debug(w);
        attributes.ast_debug(w);
        w.write(&format!("const#{index} {}:", name));
        signature.ast_debug(w);
        w.write(" = ");
        value.ast_debug(w);
        w.write(";");
    }
}

impl AstDebug for Type_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            Type_::Unit => w.write("()"),
            Type_::Multiple(ss) => {
                w.write("(");
                ss.ast_debug(w);
                w.write(")")
            }
            Type_::Apply(m, ss) => {
                m.ast_debug(w);
                if !ss.is_empty() {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
            }
            Type_::Ref(mut_, s) => {
                w.write("&");
                if *mut_ {
                    w.write("mut ");
                }
                s.ast_debug(w)
            }
            Type_::Fun(args, result) => {
                w.write("(");
                w.comma(args, |w, ty| ty.ast_debug(w));
                w.write("):");
                result.ast_debug(w);
            }
            Type_::UnresolvedError => w.write("_|_"),
        }
    }
}

impl AstDebug for Vec<(Name, AbilitySet)> {
    fn ast_debug(&self, w: &mut AstWriter) {
        if !self.is_empty() {
            w.write("<");
            w.comma(self, |w, tp| tp.ast_debug(w));
            w.write(">")
        }
    }
}

impl AstDebug for Vec<StructTypeParameter> {
    fn ast_debug(&self, w: &mut AstWriter) {
        if !self.is_empty() {
            w.write("<");
            w.comma(self, |w, tp| tp.ast_debug(w));
            w.write(">")
        }
    }
}

pub fn ability_constraints_ast_debug(w: &mut AstWriter, abilities: &AbilitySet) {
    if !abilities.is_empty() {
        w.write(": ");
        w.list(abilities, "+", |w, ab| {
            ab.ast_debug(w);
            false
        })
    }
}

impl AstDebug for (Name, AbilitySet) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (n, abilities) = self;
        w.write(n.value);
        ability_constraints_ast_debug(w, abilities)
    }
}

impl AstDebug for StructTypeParameter {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Self {
            is_phantom,
            name,
            constraints,
        } = self;
        if *is_phantom {
            w.write("phantom ");
        }
        w.write(name.value);
        ability_constraints_ast_debug(w, constraints)
    }
}

impl AstDebug for Vec<Type> {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.comma(self, |w, s| s.ast_debug(w))
    }
}

impl AstDebug for ModuleAccess_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(&match self {
            ModuleAccess_::Name(n) => format!("{}", n),
            ModuleAccess_::ModuleAccess(m, n) => format!("{}::{}", m, n),
        })
    }
}

impl AstDebug for Sequence {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.block(|w| {
            let (use_funs, items) = self;
            use_funs.ast_debug(w);
            w.semicolon(items, |w, item| item.ast_debug(w))
        })
    }
}

impl AstDebug for SequenceItem_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use SequenceItem_ as I;
        match self {
            I::Seq(e) => e.ast_debug(w),
            I::Declare(sp!(_, bs), ty_opt) => {
                w.write("let ");
                bs.ast_debug(w);
                if let Some(ty) = ty_opt {
                    ty.ast_debug(w)
                }
            }
            I::Bind(sp!(_, bs), e) => {
                w.write("let ");
                bs.ast_debug(w);
                w.write(" = ");
                e.ast_debug(w);
            }
        }
    }
}

impl AstDebug for Value_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use Value_ as V;
        match self {
            V::Address(addr) => w.write(&format!("@{}", addr)),
            V::InferredNum(u) => w.write(&format!("{}", u)),
            V::U8(u) => w.write(&format!("{}u8", u)),
            V::U16(u) => w.write(&format!("{}u16", u)),
            V::U32(u) => w.write(&format!("{}u32", u)),
            V::U64(u) => w.write(&format!("{}u64", u)),
            V::U128(u) => w.write(&format!("{}u128", u)),
            V::U256(u) => w.write(&format!("{}u256", u)),
            V::Bool(b) => w.write(&format!("{}", b)),
            V::Bytearray(v) => w.write(&format!("{:?}", v)),
        }
    }
}

impl AstDebug for Exp_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use Exp_ as E;
        match self {
            E::Unit { trailing } if !trailing => w.write("()"),
            E::Unit {
                trailing: _trailing,
            } => w.write("/*()*/"),
            E::Value(v) => v.ast_debug(w),
            E::Move(v) => w.write(&format!("move {}", v)),
            E::Copy(v) => w.write(&format!("copy {}", v)),
            E::Name(ma, tys_opt) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
            }
            E::Call(ma, is_macro, tys_opt, sp!(_, rhs)) => {
                ma.ast_debug(w);
                if *is_macro {
                    w.write("!");
                }
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("(");
                w.comma(rhs, |w, e| e.ast_debug(w));
                w.write(")");
            }
            E::MethodCall(e, f, tys_opt, sp!(_, rhs)) => {
                e.ast_debug(w);
                w.write(&format!(".{}", f));
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("(");
                w.comma(rhs, |w, e| e.ast_debug(w));
                w.write(")");
            }
            E::Pack(ma, tys_opt, fields) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("{");
                w.comma(fields, |w, (_, f, idx_e)| {
                    let (idx, e) = idx_e;
                    w.write(&format!("{}#{}: ", idx, f));
                    e.ast_debug(w);
                });
                w.write("}");
            }
            E::Vector(_loc, tys_opt, sp!(_, elems)) => {
                w.write("vector");
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("[");
                w.comma(elems, |w, e| e.ast_debug(w));
                w.write("]");
            }
            E::IfElse(b, t, f) => {
                w.write("if (");
                b.ast_debug(w);
                w.write(") ");
                t.ast_debug(w);
                w.write(" else ");
                f.ast_debug(w);
            }
            E::While(b, e) => {
                w.write("while (");
                b.ast_debug(w);
                w.write(")");
                e.ast_debug(w);
            }
            E::Loop(e) => {
                w.write("loop ");
                e.ast_debug(w);
            }
            E::Block(seq) => seq.ast_debug(w),
            E::Lambda(sp!(_, bs), e) => {
                w.write("fun ");
                bs.ast_debug(w);
                w.write(" ");
                e.ast_debug(w);
            }
            E::Quant(kind, sp!(_, rs), trs, c_opt, e) => {
                kind.ast_debug(w);
                w.write(" ");
                rs.ast_debug(w);
                trs.ast_debug(w);
                if let Some(c) = c_opt {
                    w.write(" where ");
                    c.ast_debug(w);
                }
                w.write(" : ");
                e.ast_debug(w);
            }
            E::ExpList(es) => {
                w.write("(");
                w.comma(es, |w, e| e.ast_debug(w));
                w.write(")");
            }

            E::Assign(sp!(_, lvalues), rhs) => {
                lvalues.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            }
            E::FieldMutate(ed, rhs) => {
                ed.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            }
            E::Mutate(lhs, rhs) => {
                w.write("*");
                lhs.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            }

            E::Return(e) => {
                w.write("return ");
                e.ast_debug(w);
            }
            E::Abort(e) => {
                w.write("abort ");
                e.ast_debug(w);
            }
            E::Break => w.write("break"),
            E::Continue => w.write("continue"),
            E::Dereference(e) => {
                w.write("*");
                e.ast_debug(w)
            }
            E::UnaryExp(op, e) => {
                op.ast_debug(w);
                w.write(" ");
                e.ast_debug(w);
            }
            E::BinopExp(l, op, r) => {
                l.ast_debug(w);
                w.write(" ");
                op.ast_debug(w);
                w.write(" ");
                r.ast_debug(w)
            }
            E::Borrow(mut_, e) => {
                w.write("&");
                if *mut_ {
                    w.write("mut ");
                }
                e.ast_debug(w);
            }
            E::ExpDotted(ed) => ed.ast_debug(w),
            E::Cast(e, ty) => {
                w.write("(");
                e.ast_debug(w);
                w.write(" as ");
                ty.ast_debug(w);
                w.write(")");
            }
            E::Index(oper, index) => {
                oper.ast_debug(w);
                w.write("[");
                index.ast_debug(w);
                w.write("]");
            }
            E::Annotate(e, ty) => {
                w.write("(");
                e.ast_debug(w);
                w.write(": ");
                ty.ast_debug(w);
                w.write(")");
            }
            E::Spec(u, unbound_names) => {
                w.write(&format!("spec #{}", u));
                if !unbound_names.is_empty() {
                    w.write("uses [");
                    w.comma(unbound_names, |w, n| w.write(&format!("{}", n)));
                    w.write("]");
                }
            }
            E::UnresolvedError => w.write("_|_"),
        }
    }
}

impl AstDebug for ExpDotted_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use ExpDotted_ as D;
        match self {
            D::Exp(e) => e.ast_debug(w),
            D::Dot(e, n) => {
                e.ast_debug(w);
                w.write(&format!(".{}", n))
            }
        }
    }
}

impl AstDebug for Vec<LValue> {
    fn ast_debug(&self, w: &mut AstWriter) {
        let parens = self.len() != 1;
        if parens {
            w.write("(");
        }
        w.comma(self, |w, b| b.ast_debug(w));
        if parens {
            w.write(")");
        }
    }
}

impl AstDebug for LValue_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use LValue_ as L;
        match self {
            L::Var(mutability, v, tys_opt) => {
                if mutability.is_some() {
                    w.write("mut ");
                }
                w.write(&format!("{}", v));
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
            }
            L::Unpack(ma, tys_opt, field_binds) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                field_binds.ast_debug(w);
            }
        }
    }
}

impl AstDebug for Vec<LValueWithRange> {
    fn ast_debug(&self, w: &mut AstWriter) {
        let parens = self.len() != 1;
        if parens {
            w.write("(");
        }
        w.comma(self, |w, b| b.ast_debug(w));
        if parens {
            w.write(")");
        }
    }
}

impl AstDebug for (LValue, Exp) {
    fn ast_debug(&self, w: &mut AstWriter) {
        self.0.ast_debug(w);
        w.write(" in ");
        self.1.ast_debug(w);
    }
}

impl AstDebug for Vec<Vec<Exp>> {
    fn ast_debug(&self, w: &mut AstWriter) {
        for trigger in self {
            w.write("{");
            w.comma(trigger, |w, b| b.ast_debug(w));
            w.write("}");
        }
    }
}

impl AstDebug for FieldBindings {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            FieldBindings::Named(fields) => {
                w.write("{");
                w.comma(fields, |w, (_, f, idx_b)| {
                    let (idx, b) = idx_b;
                    w.write(&format!("{}#{}: ", idx, f));
                    b.ast_debug(w);
                });
                w.write("}");
            }
            FieldBindings::Positional(vals) => {
                w.write("(");
                w.comma(vals.iter().enumerate(), |w, (idx, lval)| {
                    w.write(&format!("{idx}: "));
                    lval.ast_debug(w);
                });
                w.write(")");
            }
        }
    }
}
