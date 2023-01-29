
<a name="0x1_groups"></a>

# Module `0x1::groups`



-  [Struct `BLS12_381_G1`](#0x1_groups_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_groups_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_groups_BLS12_381_Gt)
-  [Struct `BLS12_381_Fr`](#0x1_groups_BLS12_381_Fr)
-  [Struct `SHA256`](#0x1_groups_SHA256)
-  [Struct `Scalar`](#0x1_groups_Scalar)
-  [Struct `Element`](#0x1_groups_Element)
-  [Constants](#@Constants_0)
-  [Function `pairing`](#0x1_groups_pairing)
-  [Function `pairing_product`](#0x1_groups_pairing_product)
-  [Function `scalar_from_u64`](#0x1_groups_scalar_from_u64)
-  [Function `scalar_neg`](#0x1_groups_scalar_neg)
-  [Function `scalar_add`](#0x1_groups_scalar_add)
-  [Function `scalar_mul`](#0x1_groups_scalar_mul)
-  [Function `scalar_inv`](#0x1_groups_scalar_inv)
-  [Function `scalar_eq`](#0x1_groups_scalar_eq)
-  [Function `group_identity`](#0x1_groups_group_identity)
-  [Function `group_generator`](#0x1_groups_group_generator)
-  [Function `element_neg`](#0x1_groups_element_neg)
-  [Function `element_add`](#0x1_groups_element_add)
-  [Function `element_double`](#0x1_groups_element_double)
-  [Function `element_scalar_mul`](#0x1_groups_element_scalar_mul)
-  [Function `hash_to_element`](#0x1_groups_hash_to_element)
-  [Function `element_multi_scalar_mul`](#0x1_groups_element_multi_scalar_mul)
-  [Function `scalar_deserialize`](#0x1_groups_scalar_deserialize)
-  [Function `scalar_serialize`](#0x1_groups_scalar_serialize)
-  [Function `serialize_element_uncompressed`](#0x1_groups_serialize_element_uncompressed)
-  [Function `serialize_element_compressed`](#0x1_groups_serialize_element_compressed)
-  [Function `deserialize_element_uncompressed`](#0x1_groups_deserialize_element_uncompressed)
-  [Function `deserialize_element_compressed`](#0x1_groups_deserialize_element_compressed)
-  [Function `element_eq`](#0x1_groups_element_eq)
-  [Function `group_order`](#0x1_groups_group_order)
-  [Function `abort_if_generic_group_basic_operations_disabled`](#0x1_groups_abort_if_generic_group_basic_operations_disabled)
-  [Function `abort_unless_structure_enabled`](#0x1_groups_abort_unless_structure_enabled)
-  [Function `abort_unless_hash_alg_enabled`](#0x1_groups_abort_unless_hash_alg_enabled)
-  [Function `element_deserialize_uncompressed_internal`](#0x1_groups_element_deserialize_uncompressed_internal)
-  [Function `element_deserialize_compressed_internal`](#0x1_groups_element_deserialize_compressed_internal)
-  [Function `scalar_from_u64_internal`](#0x1_groups_scalar_from_u64_internal)
-  [Function `scalar_deserialize_internal`](#0x1_groups_scalar_deserialize_internal)
-  [Function `scalar_neg_internal`](#0x1_groups_scalar_neg_internal)
-  [Function `scalar_add_internal`](#0x1_groups_scalar_add_internal)
-  [Function `scalar_double_internal`](#0x1_groups_scalar_double_internal)
-  [Function `scalar_mul_internal`](#0x1_groups_scalar_mul_internal)
-  [Function `scalar_inv_internal`](#0x1_groups_scalar_inv_internal)
-  [Function `scalar_eq_internal`](#0x1_groups_scalar_eq_internal)
-  [Function `scalar_serialize_internal`](#0x1_groups_scalar_serialize_internal)
-  [Function `element_add_internal`](#0x1_groups_element_add_internal)
-  [Function `element_eq_internal`](#0x1_groups_element_eq_internal)
-  [Function `group_identity_internal`](#0x1_groups_group_identity_internal)
-  [Function `group_order_internal`](#0x1_groups_group_order_internal)
-  [Function `group_generator_internal`](#0x1_groups_group_generator_internal)
-  [Function `element_mul_internal`](#0x1_groups_element_mul_internal)
-  [Function `element_double_internal`](#0x1_groups_element_double_internal)
-  [Function `element_neg_internal`](#0x1_groups_element_neg_internal)
-  [Function `element_serialize_uncompressed_internal`](#0x1_groups_element_serialize_uncompressed_internal)
-  [Function `element_serialize_compressed_internal`](#0x1_groups_element_serialize_compressed_internal)
-  [Function `element_multi_scalar_mul_internal`](#0x1_groups_element_multi_scalar_mul_internal)
-  [Function `pairing_product_internal`](#0x1_groups_pairing_product_internal)
-  [Function `hash_to_element_internal`](#0x1_groups_hash_to_element_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a name="0x1_groups_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`

<code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code> represents a group used in BLS12-381 pairing.
<code>Fq</code> is a finite field with <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.
<code>E(Fq)</code> is an elliptic curve <code>y^2=x^3+4</code> defined over <code>Fq</code>.
<code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code> is constructed by a subset of the points on <code>E(Fq)</code> and the point at infinity, under point addition. (A subgroup of prime order on <code>E(Fq)</code>.)
The prime order <code>r</code> of <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code> is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
The identity of <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code> is the point at infinity.
There exists a bilinear mapping from <code>(<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>, <a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>)</code> to <code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code>.

A <code><a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> is an integer between 0 and <code>r-1</code>.

An <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> represents an element in group <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code>.
Scalar multiplication on <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> requires a <code><a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>.

Functions <code><a href="groups.md#0x1_groups_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> and <code><a href="groups.md#0x1_groups_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>
assume a 96-byte encoding <code>[b_0, ..., b_95]</code> for <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>, with the following rules.
- <code>b_95 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq)</code>, with the following rules.
- <code>[b_0, ..., b_47 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x</code>.
- <code>[b_48, ..., b_95 & 0x3f]</code> is a 48-byte little-endian encoding of 'y'.

Functions <code><a href="groups.md#0x1_groups_serialize_element_compressed">serialize_element_compressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> and <code><a href="groups.md#0x1_groups_deserialize_element_compressed">deserialize_element_compressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>
assume a 48-byte encoding <code>[b_0, ..., b_47]</code> of an <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> with the following rules.
- <code>b_47 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve, with the following rules.
- <code>[b_0, ..., b_47 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x</code>.
- <code>b_47 & 0x80</code> is the positiveness flag.
- The positiveness flag is 1 if and only if <code>y &gt; -y</code>.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groups_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`

<code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code> represents a group used in BLS12-381 pairing.
<code>Fq</code> is a finite field with <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.
<code>Fq2</code> is an extension field of <code>Fq</code>, constructed as <code>Fq2=Fq[u]/(u^2+1)</code>.
<code>E(Fq2)</code> is an elliptic curve <code>y^2=x^3+4(u+1)</code> defined over <code>Fq2</code>.
<code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code> is constructed by a subset of the points on <code>E(Fq2)</code> and the point at infinity, under point addition. (A subgroup of prime order on <code>E(Fq2)</code>.)
The prime order <code>r</code> of <code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code> is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001, same as <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code>.
The identity of <code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code> is the point at infinity.
There exists a bilinear mapping from <code>(<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>, <a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>)</code> to <code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code>.

An <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> is an element in group <code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code>.
Scalar multiplication on <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> requires a <code><a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>.

Functions <code><a href="groups.md#0x1_groups_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> and <code><a href="groups.md#0x1_groups_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>
assume a 192-byte encoding <code>[b_0, ..., b_191]</code> of an <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>, with the following rules.
- <code>b_191 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>, with the following rules.
- <code>[b_0, ..., b_95]</code> is a 96-byte encoding of <code>x=(x_0+x_1*u)</code>.
- <code>[b_0, ..., b_47]</code> is a 48-byte little-endian encoding of <code>x_0</code>.
- <code>[b_48, ..., b_95]</code> is a 48-byte little-endian encoding of <code>x_1</code>.
- <code>[b_96, ..., b_191 & 0x3f]</code> is a 96-byte encoding of 'y=(y_0+y_1*u)'.
- <code>[b_96, ..., b_143]</code> is a 48-byte little-endian encoding of <code>y_0</code>.
- <code>[b_144, ..., b_191 & 0x3f]</code> is a 48-byte little-endian encoding of <code>y_1</code>.

Functions <code><a href="groups.md#0x1_groups_serialize_element_compressed">serialize_element_compressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> and <code><a href="groups.md#0x1_groups_deserialize_element_compressed">deserialize_element_compressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>
assume a 96-byte encoding <code>[b_0, ..., b_95]</code> of an <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> with the following rules.
- <code>b_95 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>, with the following rules.
- <code>[b_0, ..., b_95 & 0x3f]</code> is a 96-byte little-endian encoding of <code>x=(x_0+x_1*u)</code>.
- <code>[b_0, ..., b_47]</code> is a 48-byte little-endian encoding of <code>x_0</code>.
- <code>[b_48, ..., b_95 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x_1</code>.
- <code>b_95 & 0x80</code> is the positiveness flag.
- The positiveness flag is 1 if and only if <code>y &gt; -y</code>.
- Here <code>a=(a_0+a_1*u)</code> is considered greater than <code>b=(b_0+b_1*u)</code> if <code>a_1&gt;b_1 OR (a_1=b_1 AND a_0&gt;b_0)</code>.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groups_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`

<code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code> represents the target group of the pairing defined over the BLS12-381 curves.
<code>Fq</code> is a finite field with <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.
<code>Fq2</code> is an extension field of <code>Fq</code>, constructed as <code>Fq2=Fq[u]/(u^2+1)</code>.
<code>Fq6</code> is an extension field of <code>Fq2</code>, constructed as <code>Fq6=Fq2[v]/(v^2-u-1)</code>.
<code>Fq12</code> is an extension field of <code>Fq6</code>, constructed as <code>Fq12=Fq6[w]/(w^2-v)</code>.
<code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code> is a multiplicative subgroup of <code>Fq12</code>.
The order <code>r</code> of <code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code> is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001. (Same as <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code> and <code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code>.)
The identity of <code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code> is 1.
There exists a bilinear mapping from <code>(<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>, <a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>)</code> to <code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code>.

An <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> is an element in group <code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code>.
Scalar multiplication on <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> requires a <code><a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>.

Functions <code><a href="groups.md#0x1_groups_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> and <code><a href="groups.md#0x1_groups_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>,
as well as <code>serialize_element_ompressed&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> and <code><a href="groups.md#0x1_groups_deserialize_element_compressed">deserialize_element_compressed</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>,
assume a 576-byte encoding <code>[b_0, ..., b_575]</code> of an <code><a href="groups.md#0x1_groups_Element">Element</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>, with the following rules.
- Assume the given element is <code>e=c_0+c_1*w</code> where <code>c_i=c_i0+c_i1*v+c_i2*v^2 for i=0..1</code> and <code>c_ij=c_ij0+c_ij1*u for j=0..2</code>.
- <code>[b_0, ..., b_575]</code> is a concatenation of 12 encoded <code>Fq</code> elements: <code>c_000, c_001, c_010, c_011, c_020, c_021, c_100, c_101, c_110, c_111, c_120, c_121</code>.
- Every <code>c_ijk</code> uses a 48-byte little-endian encoding.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groups_BLS12_381_Fr"></a>

## Struct `BLS12_381_Fr`

The scalar field for groups <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code>, <code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code> and <code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code>.
A <code><a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code> is an integer between 0 and <code>r-1</code> where <code>r</code> is the order of <code><a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a></code>/<code><a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a></code>/<code><a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a></code>.
Functions <code>deserialize_scalar&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code> and <code>serialize_scalar&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>
assume a 32-byte little-endian encoding of a <code><a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groups_SHA256"></a>

## Struct `SHA256`



<pre><code><b>struct</b> <a href="groups.md#0x1_groups_SHA256">SHA256</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groups_Scalar"></a>

## Struct `Scalar`

This struct represents an integer between 0 and <code>r-1</code>, where <code>r</code> is the order of group <code>G</code>.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;G&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groups_Element"></a>

## Struct `Element`

This struct represents an element of the group represented by the type argument <code>G</code>.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_groups_E_NOT_IMPLEMENTED"></a>



<pre><code><b>const</b> <a href="groups.md#0x1_groups_E_NOT_IMPLEMENTED">E_NOT_IMPLEMENTED</a>: u64 = 2;
</code></pre>



<a name="0x1_groups_pairing"></a>

## Function `pairing`

Perform a pairing.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing">pairing</a>&lt;G1, G2, Gt&gt;(element_1: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G1&gt;, element_2: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G2&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing">pairing</a>&lt;G1,G2,Gt&gt;(element_1: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G1&gt;, element_2: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G2&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;Gt&gt; {
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[element_1.handle], <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[element_2.handle])
    }
}
</code></pre>



</details>

<a name="0x1_groups_pairing_product"></a>

## Function `pairing_product`

Compute the product of multiple pairing.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing_product">pairing_product</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G2&gt;&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing_product">pairing_product</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Element">Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Element">Element</a>&lt;G2&gt;&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;Gt&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G1&gt;();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G2&gt;();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;Gt&gt;();
    <b>let</b> num_g1 = std::vector::length(g1_elements);
    <b>let</b> num_g2 = std::vector::length(g2_elements);
    <b>assert</b>!(num_g1 == num_g2, std::error::invalid_argument(1));
    <b>let</b> g1_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> g2_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_g2) {
        std::vector::push_back(&<b>mut</b> g1_handles, std::vector::borrow(g1_elements, i).handle);
        std::vector::push_back(&<b>mut</b> g2_handles, std::vector::borrow(g2_elements, i).handle);
        i = i + 1;
    };

    <a href="groups.md#0x1_groups_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(g1_handles, g2_handles)
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_from_u64"></a>

## Function `scalar_from_u64`

Convert a u64 to a scalar.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_from_u64">scalar_from_u64</a>&lt;S&gt;(value: u64): <a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_from_u64">scalar_from_u64</a>&lt;S&gt;(value: u64): <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
        handle: <a href="groups.md#0x1_groups_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;S&gt;(value)
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_neg"></a>

## Function `scalar_neg`

Compute <code>-x</code> for scalar <code>x</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_neg">scalar_neg</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_neg">scalar_neg</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
        handle: <a href="groups.md#0x1_groups_scalar_neg_internal">scalar_neg_internal</a>&lt;S&gt;(x.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_add"></a>

## Function `scalar_add`

Compute <code>x + y</code> for scalar <code>x</code> and <code>y</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_add">scalar_add</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;, y: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_add">scalar_add</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;, y: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
        handle: <a href="groups.md#0x1_groups_scalar_add_internal">scalar_add_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_mul"></a>

## Function `scalar_mul`

Compute <code>x * y</code> for scalar <code>x</code> and <code>y</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_mul">scalar_mul</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;, y: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_mul">scalar_mul</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;, y: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
        handle: <a href="groups.md#0x1_groups_scalar_mul_internal">scalar_mul_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_inv"></a>

## Function `scalar_inv`

Compute <code>x^(-1)</code> for scalar <code>x</code>, if defined.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_inv">scalar_inv</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_inv">scalar_inv</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): Option&lt;<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> (succeeded, handle) = <a href="groups.md#0x1_groups_scalar_inv_internal">scalar_inv_internal</a>&lt;S&gt;(x.handle);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; { handle };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_eq"></a>

## Function `scalar_eq`

Check if <code>x == y</code> for scalar <code>x</code> and <code>y</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_eq">scalar_eq</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;, y: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_eq">scalar_eq</a>&lt;S&gt;(x: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;, y: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): bool {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_scalar_eq_internal">scalar_eq_internal</a>&lt;S&gt;(x.handle, y.handle)
}
</code></pre>



</details>

<a name="0x1_groups_group_identity"></a>

## Function `group_identity`

Get the identity of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_group_identity">group_identity</a>&lt;G&gt;(): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_group_identity">group_identity</a>&lt;G&gt;(): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_group_identity_internal">group_identity_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_groups_group_generator"></a>

## Function `group_generator`

Get the generator of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_group_generator">group_generator</a>&lt;G&gt;(): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_group_generator">group_generator</a>&lt;G&gt;(): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_group_generator_internal">group_generator_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_neg"></a>

## Function `element_neg`

Compute <code>-P</code> for group element <code>P</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_neg">element_neg</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_neg">element_neg</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_element_neg_internal">element_neg_internal</a>&lt;G&gt;(element_p.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_add"></a>

## Function `element_add`

Compute <code>P + Q</code> for group element <code>P</code> and <code>Q</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_add">element_add</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_add">element_add</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_element_add_internal">element_add_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_double"></a>

## Function `element_double`

Compute <code>2P</code> for group element <code>P</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_double">element_double</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_double">element_double</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_element_double_internal">element_double_internal</a>&lt;G&gt;(element_p.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_scalar_mul"></a>

## Function `element_scalar_mul`

Compute <code>k*P</code> for scalar <code>k</code> and group element <code>P</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_scalar_mul">element_scalar_mul</a>&lt;G, S&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;, scalar_k: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_scalar_mul">element_scalar_mul</a>&lt;G, S&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;, scalar_k: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_element_mul_internal">element_mul_internal</a>&lt;G, S&gt;(element_p.handle, scalar_k.handle)
    }
}
</code></pre>



</details>

<a name="0x1_groups_hash_to_element"></a>

## Function `hash_to_element`

Hash bytes to a group element.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_hash_to_element">hash_to_element</a>&lt;H, G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_hash_to_element">hash_to_element</a>&lt;H, G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_hash_alg_enabled">abort_unless_hash_alg_enabled</a>&lt;H&gt;();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_hash_to_element_internal">hash_to_element_internal</a>&lt;H, G&gt;(bytes)
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_multi_scalar_mul"></a>

## Function `element_multi_scalar_mul`

Compute <code>k[0]*P[0]+...+k[n-1]*P[n-1]</code> for a list of scalars <code>k[]</code> and a list of group elements <code>P[]</code>, both of size <code>n</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_multi_scalar_mul">element_multi_scalar_mul</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_multi_scalar_mul">element_multi_scalar_mul</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> num_scalars = std::vector::length(scalars);
    <b>let</b> scalar_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_scalars) {
        std::vector::push_back(&<b>mut</b> scalar_handles, std::vector::borrow(scalars, i).handle);
        i = i + 1;
    };

    <b>let</b> num_elements = std::vector::length(elements);
    <b>let</b> element_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_elements) {
        std::vector::push_back(&<b>mut</b> element_handles, std::vector::borrow(elements, i).handle);
        i = i + 1;
    };

    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_element_multi_scalar_mul_internal">element_multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles, scalar_handles)
    }

}
</code></pre>



</details>

<a name="0x1_groups_scalar_deserialize"></a>

## Function `scalar_deserialize`

Scalar deserialization.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_deserialize">scalar_deserialize</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_deserialize">scalar_deserialize</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> (succeeded, handle) = <a href="groups.md#0x1_groups_scalar_deserialize_internal">scalar_deserialize_internal</a>&lt;S&gt;(*bytes);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt; {
            handle
        };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_groups_scalar_serialize"></a>

## Function `scalar_serialize`

Scalar serialization.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_serialize">scalar_serialize</a>&lt;S&gt;(scalar: &<a href="groups.md#0x1_groups_Scalar">groups::Scalar</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_serialize">scalar_serialize</a>&lt;S&gt;(scalar: &<a href="groups.md#0x1_groups_Scalar">Scalar</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="groups.md#0x1_groups_scalar_serialize_internal">scalar_serialize_internal</a>&lt;S&gt;(scalar.handle)
}
</code></pre>



</details>

<a name="0x1_groups_serialize_element_uncompressed"></a>

## Function `serialize_element_uncompressed`

Group element serialization with an uncompressed format.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;G&gt;(element: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;G&gt;(element: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_element_serialize_uncompressed_internal">element_serialize_uncompressed_internal</a>&lt;G&gt;(element.handle)
}
</code></pre>



</details>

<a name="0x1_groups_serialize_element_compressed"></a>

## Function `serialize_element_compressed`

Group element serialization with a compressed format.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_serialize_element_compressed">serialize_element_compressed</a>&lt;G&gt;(element: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_serialize_element_compressed">serialize_element_compressed</a>&lt;G&gt;(element: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_element_serialize_compressed_internal">element_serialize_compressed_internal</a>&lt;G&gt;(element.handle)
}
</code></pre>



</details>

<a name="0x1_groups_deserialize_element_uncompressed"></a>

## Function `deserialize_element_uncompressed`

Group element deserialization with an uncompressed format.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <b>let</b> (succ, handle) = <a href="groups.md#0x1_groups_element_deserialize_uncompressed_internal">element_deserialize_uncompressed_internal</a>&lt;G&gt;(bytes);
    <b>if</b> (succ) {
        std::option::some(<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; { handle })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_groups_deserialize_element_compressed"></a>

## Function `deserialize_element_compressed`

Group element deserialization with a compressed format.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_deserialize_element_compressed">deserialize_element_compressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_deserialize_element_compressed">deserialize_element_compressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <b>let</b> (succ, handle) = <a href="groups.md#0x1_groups_element_deserialize_compressed_internal">element_deserialize_compressed_internal</a>&lt;G&gt;(bytes);
    <b>if</b> (succ) {
        std::option::some(<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; { handle })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_eq"></a>

## Function `element_eq`

Check if <code>P == Q</code> for group elements <code>P</code> and <code>Q</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_eq">element_eq</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_element_eq">element_eq</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): bool {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_element_eq_internal">element_eq_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
}
</code></pre>



</details>

<a name="0x1_groups_group_order"></a>

## Function `group_order`

Get the order of group <code>G</code>, little-endian encoded as a byte string.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_group_order">group_order</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_group_order">group_order</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="groups.md#0x1_groups_group_order_internal">group_order_internal</a>&lt;G&gt;()
}
</code></pre>



</details>

<a name="0x1_groups_abort_if_generic_group_basic_operations_disabled"></a>

## Function `abort_if_generic_group_basic_operations_disabled`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="groups.md#0x1_groups_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>() {
    <b>if</b> (!std::features::generic_group_basic_operations_enabled()) {
        <b>abort</b>(std::error::not_implemented(0))
    }
}
</code></pre>



</details>

<a name="0x1_groups_abort_unless_structure_enabled"></a>

## Function `abort_unless_structure_enabled`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="groups.md#0x1_groups_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;() {
    <b>let</b> type = type_of&lt;S&gt;();
    <b>if</b> ((type == type_of&lt;<a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>&gt;() || type == type_of&lt;<a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>&gt;() || type == type_of&lt;<a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>&gt;() || type == type_of&lt;<a href="groups.md#0x1_groups_BLS12_381_Fr">BLS12_381_Fr</a>&gt;())
        && std::features::bls12_381_groups_enabled()
    ) {
        // Let go.
    } <b>else</b> {
        <b>abort</b>(std::error::not_implemented(0))
    }
}
</code></pre>



</details>

<a name="0x1_groups_abort_unless_hash_alg_enabled"></a>

## Function `abort_unless_hash_alg_enabled`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_abort_unless_hash_alg_enabled">abort_unless_hash_alg_enabled</a>&lt;S&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="groups.md#0x1_groups_abort_unless_hash_alg_enabled">abort_unless_hash_alg_enabled</a>&lt;S&gt;() {
    <b>let</b> type = type_of&lt;S&gt;();
    <b>if</b> (type == type_of&lt;<a href="groups.md#0x1_groups_SHA256">SHA256</a>&gt;() && std::features::sha256_to_group_enabled()) {
        // Let go.
    } <b>else</b> {
        <b>abort</b>(std::error::not_implemented(0))
    }
}
</code></pre>



</details>

<a name="0x1_groups_element_deserialize_uncompressed_internal"></a>

## Function `element_deserialize_uncompressed_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_deserialize_uncompressed_internal">element_deserialize_uncompressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_deserialize_uncompressed_internal">element_deserialize_uncompressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_groups_element_deserialize_compressed_internal"></a>

## Function `element_deserialize_compressed_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_deserialize_compressed_internal">element_deserialize_compressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_deserialize_compressed_internal">element_deserialize_compressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_groups_scalar_from_u64_internal"></a>

## Function `scalar_from_u64_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_scalar_deserialize_internal"></a>

## Function `scalar_deserialize_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_deserialize_internal">scalar_deserialize_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_deserialize_internal">scalar_deserialize_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_groups_scalar_neg_internal"></a>

## Function `scalar_neg_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_scalar_add_internal"></a>

## Function `scalar_add_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_scalar_double_internal"></a>

## Function `scalar_double_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_double_internal">scalar_double_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_double_internal">scalar_double_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_scalar_inv_internal"></a>

## Function `scalar_inv_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(handle: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(handle: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_groups_scalar_eq_internal"></a>

## Function `scalar_eq_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_groups_scalar_serialize_internal"></a>

## Function `scalar_serialize_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_scalar_serialize_internal">scalar_serialize_internal</a>&lt;G&gt;(h: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_scalar_serialize_internal">scalar_serialize_internal</a>&lt;G&gt;(h: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_groups_element_add_internal"></a>

## Function `element_add_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_add_internal">element_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_add_internal">element_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_element_eq_internal"></a>

## Function `element_eq_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_eq_internal">element_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_eq_internal">element_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_groups_group_identity_internal"></a>

## Function `group_identity_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_group_identity_internal">group_identity_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_group_identity_internal">group_identity_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_groups_group_order_internal"></a>

## Function `group_order_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_group_order_internal">group_order_internal</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_group_order_internal">group_order_internal</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_groups_group_generator_internal"></a>

## Function `group_generator_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_group_generator_internal">group_generator_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_group_generator_internal">group_generator_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_groups_element_mul_internal"></a>

## Function `element_mul_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_mul_internal">element_mul_internal</a>&lt;G, S&gt;(scalar_handle: u64, element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_mul_internal">element_mul_internal</a>&lt;G, S&gt;(scalar_handle: u64, element_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_element_double_internal"></a>

## Function `element_double_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_double_internal">element_double_internal</a>&lt;G&gt;(element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_double_internal">element_double_internal</a>&lt;G&gt;(element_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_element_neg_internal"></a>

## Function `element_neg_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_neg_internal">element_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_neg_internal">element_neg_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_groups_element_serialize_uncompressed_internal"></a>

## Function `element_serialize_uncompressed_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_serialize_uncompressed_internal">element_serialize_uncompressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_serialize_uncompressed_internal">element_serialize_uncompressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_groups_element_serialize_compressed_internal"></a>

## Function `element_serialize_compressed_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_serialize_compressed_internal">element_serialize_compressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_serialize_compressed_internal">element_serialize_compressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_groups_element_multi_scalar_mul_internal"></a>

## Function `element_multi_scalar_mul_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_element_multi_scalar_mul_internal">element_multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_element_multi_scalar_mul_internal">element_multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>

<a name="0x1_groups_pairing_product_internal"></a>

## Function `pairing_product_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1, G2, Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>

<a name="0x1_groups_hash_to_element_internal"></a>

## Function `hash_to_element_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_hash_to_element_internal">hash_to_element_internal</a>&lt;H, G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_hash_to_element_internal">hash_to_element_internal</a>&lt;H, G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
