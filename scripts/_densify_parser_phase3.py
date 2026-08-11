#!/usr/bin/env python3
"""P6b phase 3: densify slowest parser shards (session/stmt/postfix/hooks/main).

Run from repo root (idempotent via markers):
  python scripts/_densify_parser_phase3.py
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SH = ROOT / "self_host"

IMPORTS = """import "self_host/lexer_defs"
import "self_host/ast_defs"
import "self_host/parser_hooks"
import "self_host/parser_util"
"""


def write(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print(f"wrote {p.relative_to(ROOT)} ({len(text.splitlines())} lines)")


def already(name: str, marker: str) -> bool:
    p = SH / name
    return p.is_file() and marker in p.read_text(encoding="utf-8")


# --- session: AccAdd density is the bottleneck ---

CORE_KEYS = [
    ("pPos", "0"),
    ("pToks", "[]"),
    ("pToksLen", "0"),
    ("pCur", "null"),
    ("pEofTok", '{ "type": "EOF", "value": null, "line": 1, "column": 1 }'),
    ("pTok", "null"),
    ("pNextTok", "null"),
    ("pDone", "0"),
    ("pSymPool", "[]"),
    ("pSymN", "0"),
    ("pSymOut", '""'),
    ("pSymSrc", '""'),
    ("pSymI", "0"),
    ("pPopI", "0"),
    ("pPopNew", "[]"),
    ("pImports", "[]"),
    ("pIsPub", "0"),
    ("pFnPub", "0"),
    ("pInAddSub", "0"),
    ("pNoBit", "0"),
]

EXPR_KEYS = [
    ("pLeft", "null"),
    ("pRight", "null"),
    ("pArgs", "[]"),
    ("pIdent", "null"),
    ("pInit", "null"),
    ("pVal", "null"),
    ("pCallee", "null"),
    ("pObjFields", "[]"),
    ("pSave", "null"),
    ("pAssignLhs", "null"),
    ("pBinOp", '""'),
    ("pSaveSym", '""'),
    ("pBindSym", '""'),
    ("pExprLeft", "null"),
    ("pAddLeft", "null"),
    ("pAddLeftStack", "[]"),
    ("pTypeArgs", "[]"),
    ("pSnapTypeArgs", "[]"),
    ("pIsGenericCallee", "0"),
    ("pAngleI", "0"),
    ("pAngleTok", "null"),
    ("pAngleOut", "[]"),
    ("pMemberName", '""'),
]

STMT_KEYS = [
    ("pBody", "[]"),
    ("pBodyStack", "[]"),
    ("pStmts", "[]"),
    ("pParams", "[]"),
    ("pCond", "null"),
    ("pCondStack", "[]"),
    ("pThen", "null"),
    ("pElse", "null"),
    ("pLoopBody", "[]"),
    ("pFnSym", '""'),
    ("pLetSym", '""'),
    ("pBodyDepth", "0"),
    ("pCondDepth", "0"),
]

TYPE_KEYS = [
    ("pTypeParams", "[]"),
    ("pParamTypes", "[]"),
    ("pReturnType", '""'),
    ("pVariants", "[]"),
    ("pClassFields", "[]"),
    ("pClassMethods", "[]"),
    ("pFieldType", '""'),
    ("pFieldDefault", "null"),
    ("pVariantFields", "[]"),
    ("pMethTypeParams", "[]"),
    ("pMethParams", "[]"),
    ("pMethParamTypes", "[]"),
    ("pVariantName", '""'),
    ("pVariantSym", '""'),
    ("pEnumSym", '""'),
    ("pClassSym", '""'),
    ("pIsStruct", "0"),
    ("pIfaceSym", '""'),
    ("pIfaceMethods", "[]"),
    ("pAssocTypes", "[]"),
    ("pWhereClause", "[]"),
    ("pWhereParam", '""'),
    ("pWhereTrait", '""'),
    ("pParamType", '""'),
]


def _assign_block(keys: list[tuple[str, str]], indent: str = "    ") -> str:
    return "\n".join(f'{indent}sess["{k}"] = {v}' for k, v in keys)


def split_session() -> None:
    if already("parser_session.kab", "parser_session_core"):
        print("skip parser_session")
        return
    for group, keys in [
        ("core", CORE_KEYS),
        ("expr", EXPR_KEYS),
        ("stmt", STMT_KEYS),
        ("type", TYPE_KEYS),
    ]:
        write(
            f"parser_session_{group}.kab",
            f"""// P6b: parser session {group} fields.
pub fn pSessionInit_{group}(sess) {{
{_assign_block(keys)}
    return 0
}}
""",
        )
    write(
        "parser_session.kab",
        """// P6b: parser session — thread sess into all shards.
import "self_host/parser_session_core"
import "self_host/parser_session_expr"
import "self_host/parser_session_stmt"
import "self_host/parser_session_type"

pub fn pMakeSession() {
    let sess = {}
    pSessionInit_core(sess)
    pSessionInit_expr(sess)
    pSessionInit_stmt(sess)
    pSessionInit_type(sess)
    return sess
}

pub fn pResetSession(sess) {
    pSessionInit_core(sess)
    pSessionInit_expr(sess)
    pSessionInit_stmt(sess)
    pSessionInit_type(sess)
    return 0
}
""",
    )


def split_stmt_class() -> None:
    if already("parser_stmt_class.kab", "parser_stmt_class_method"):
        print("skip parser_stmt_class")
        return
    write(
        "parser_stmt_class_method.kab",
        IMPORTS
        + """
pub fn parseStmt_class_method(sess) {
    if sess["pCur"].type != TOKEN_FN {
        return 0
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected method name"
    }
    bump(sess)
    sess["pFnSym"] = sess["pTok"].value
    sess["pMethTypeParams"] = pCallTypeArgs(sess)
    if sess["pCur"].type != "(" {
        throw "Expected ( at " + ("" + sess["pCur"].line) + ":" + ("" + sess["pCur"].column) + " tok=" + sess["pCur"].type
    }
    bump(sess)
    sess["pMethParams"] = []
    sess["pMethParamTypes"] = []
    if sess["pCur"].type != ")" {
        while true {
            sess["pTok"] = sess["pCur"]
            if sess["pTok"].type == TOKEN_SELF {
                bump(sess)
                sess["pMethParams"] = push(sess["pMethParams"], poolPush(sess, "self"))
                sess["pMethParamTypes"] = push(sess["pMethParamTypes"], "")
            } else {
                if sess["pTok"].type != TOKEN_IDENT {
                    throw "Expected parameter name"
                }
                bump(sess)
                sess["pMethParams"] = push(sess["pMethParams"], poolPush(sess, sess["pTok"].value))
                sess["pParamType"] = ""
                if sess["pCur"].type == ":" {
                    bump(sess)
                    sess["pTok"] = sess["pCur"]
                    if sess["pTok"].type != TOKEN_IDENT {
                        throw "Expected parameter type"
                    }
                    bump(sess)
                    sess["pParamType"] = sess["pTok"].value
                }
                sess["pMethParamTypes"] = push(sess["pMethParamTypes"], sess["pParamType"])
            }
            if sess["pCur"].type == "," {
                bump(sess)
            } else {
                break
            }
        }
    }
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == "EOF" {
            throw "Unclosed block"
        }
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    let methBody = sess["pBody"]
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    sess["pVal"] = {
        "sym": sess["pFnSym"],
        "typeParams": sess["pMethTypeParams"],
        "params": sess["pMethParams"],
        "paramTypes": sess["pMethParamTypes"],
        "returnType": "",
        "body": { "kind": AST_BLOCK, "body": methBody }
    }
    sess["pClassMethods"] = push(sess["pClassMethods"], sess["pVal"])
    return 0
}
""",
    )
    write(
        "parser_stmt_class_field.kab",
        IMPORTS
        + """
pub fn parseStmt_class_field(sess) {
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected field name"
    }
    bump(sess)
    sess["pSaveSym"] = poolPush(sess, sess["pTok"].value)
    sess["pFieldType"] = ""
    if sess["pCur"].type == ":" {
        bump(sess)
        sess["pTok"] = sess["pCur"]
        if sess["pTok"].type != TOKEN_IDENT {
            throw "Expected field type"
        }
        bump(sess)
        sess["pFieldType"] = sess["pTok"].value
    }
    sess["pFieldDefault"] = null
    if sess["pCur"].type == "=" {
        bump(sess)
        sess["pFieldDefault"] = pCallCompare(sess)
    }
    if sess["pCur"].type == ";" {
        bump(sess)
    }
    sess["pVal"] = { "name": sess["pSaveSym"], "type": sess["pFieldType"], "default": sess["pFieldDefault"] }
    sess["pClassFields"] = push(sess["pClassFields"], sess["pVal"])
    return 0
}
""",
    )
    write(
        "parser_stmt_class.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_class_method"
import "self_host/parser_stmt_class_field"

pub fn parseStmt_class(sess) {
    if !(sess["pCur"].type == TOKEN_CLASS || sess["pCur"].type == TOKEN_STRUCT) {
        return null
    }
    sess["pIsStruct"] = 0
    if sess["pCur"].type == TOKEN_STRUCT {
        sess["pIsStruct"] = 1
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        if sess["pIsStruct"] == 1 {
            throw "Expected struct name"
        }
        throw "Expected class name"
    }
    bump(sess)
    sess["pClassSym"] = sess["pTok"].value
    sess["pTypeParams"] = pCallTypeArgs(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pClassFields"] = []
    sess["pClassMethods"] = []
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == TOKEN_FN {
            parseStmt_class_method(sess)
        } else {
            parseStmt_class_field(sess)
        }
    }
    bump(sess)
    return {
        "kind": AST_CLASS,
        "sym": sess["pClassSym"],
        "typeParams": sess["pTypeParams"],
        "fields": sess["pClassFields"],
        "methods": sess["pClassMethods"],
        "isStruct": sess["pIsStruct"]
    }
}
""",
    )


def split_postfix_lit() -> None:
    if already("parser_postfix_lit.kab", "parser_postfix_lit_scalar"):
        print("skip parser_postfix_lit")
        return
    write(
        "parser_postfix_lit_scalar.kab",
        IMPORTS
        + """
pub fn parsePostfix_lit_scalar(sess) {
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type == TOKEN_NUMBER {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_NUMBER, "value": sess["pTok"].value }
        return 0
    }
    if sess["pTok"].type == TOKEN_STRING {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_STRING, "value": sess["pTok"].value }
        return 0
    }
    if sess["pTok"].type == TOKEN_TRUE {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_BOOL, "value": true }
        return 0
    }
    if sess["pTok"].type == TOKEN_FALSE {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_BOOL, "value": false }
        return 0
    }
    if sess["pTok"].type == TOKEN_NULL {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_NULL, "value": null }
        return 0
    }
    if sess["pTok"].type == TOKEN_UNDEFINED {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_UNDEF, "value": undefined }
        return 0
    }
    if sess["pTok"].type == TOKEN_NONE {
        bump(sess)
        sess["pLeft"] = { "kind": AST_VAR, "sym": "None" }
        return 0
    }
    return 0
}
""",
    )
    write(
        "parser_postfix_lit_wrap.kab",
        IMPORTS
        + """
pub fn parsePostfix_lit_wrap(sess) {
    if sess["pLeft"] != null {
        return 0
    }
    sess["pTok"] = sess["pCur"]
    if !(sess["pTok"].type == TOKEN_OK || sess["pTok"].type == TOKEN_ERR || sess["pTok"].type == TOKEN_SOME) {
        return 0
    }
    bump(sess)
    let wrapName = sess["pTok"].type
    if sess["pCur"].type != "(" {
        throw "Expected ( at " + ("" + sess["pCur"].line) + ":" + ("" + sess["pCur"].column) + " tok=" + sess["pCur"].type
    }
    bump(sess)
    let wrapInner = pCallCompare(sess)
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    sess["pLeft"] = {
        "kind": AST_CALL,
        "callee": { "kind": AST_VAR, "sym": wrapName },
        "args": [wrapInner]
    }
    return 0
}
""",
    )
    write(
        "parser_postfix_lit_ident.kab",
        IMPORTS
        + """
pub fn parsePostfix_lit_ident(sess) {
    if sess["pLeft"] != null {
        return 0
    }
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type == TOKEN_IDENT {
        bump(sess)
        sess["pLeft"] = { "kind": AST_VAR, "sym": poolPush(sess, sess["pTok"].value) }
        return 0
    }
    if sess["pTok"].type == TOKEN_SELF {
        bump(sess)
        sess["pLeft"] = { "kind": AST_VAR, "sym": "self" }
        return 0
    }
    if sess["pTok"].type == TOKEN_THIS {
        bump(sess)
        sess["pLeft"] = { "kind": AST_VAR, "sym": "this" }
        return 0
    }
    return 0
}
""",
    )
    write(
        "parser_postfix_lit.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_lit_scalar"
import "self_host/parser_postfix_lit_wrap"
import "self_host/parser_postfix_lit_ident"

pub fn parsePostfix_lit(sess) {
    sess["pLeft"] = null
    sess["pTypeArgs"] = []
    parsePostfix_lit_scalar(sess)
    parsePostfix_lit_wrap(sess)
    parsePostfix_lit_ident(sess)
    return 0
}
""",
    )


def split_stmt_fn() -> None:
    if already("parser_stmt_fn.kab", "parser_stmt_fn_sig"):
        print("skip parser_stmt_fn")
        return
    write(
        "parser_stmt_fn_sig.kab",
        IMPORTS
        + """
pub fn parseStmt_fn_sig(sess) {
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected function name"
    }
    bump(sess)
    sess["pFnSym"] = sess["pTok"].value
    sess["pFnPub"] = sess["pIsPub"]
    sess["pTypeParams"] = pCallTypeArgs(sess)
    if sess["pCur"].type != "(" {
        throw "Expected ( at " + ("" + sess["pCur"].line) + ":" + ("" + sess["pCur"].column) + " tok=" + sess["pCur"].type
    }
    bump(sess)
    sess["pParams"] = []
    sess["pParamTypes"] = []
    if sess["pCur"].type != ")" {
        while true {
            sess["pTok"] = sess["pCur"]
            if sess["pTok"].type != TOKEN_IDENT {
                throw "Expected parameter name"
            }
            bump(sess)
            sess["pParams"] = push(sess["pParams"], poolPush(sess, sess["pTok"].value))
            sess["pParamType"] = ""
            if sess["pCur"].type == ":" {
                bump(sess)
                sess["pTok"] = sess["pCur"]
                if sess["pTok"].type != TOKEN_IDENT {
                    throw "Expected parameter type"
                }
                bump(sess)
                sess["pParamType"] = sess["pTok"].value
            }
            sess["pParamTypes"] = push(sess["pParamTypes"], sess["pParamType"])
            if sess["pCur"].type == "," {
                bump(sess)
            } else {
                break
            }
        }
    }
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    sess["pReturnType"] = ""
    if sess["pCur"].type == "->" {
        bump(sess)
        sess["pTok"] = sess["pCur"]
        if sess["pTok"].type != TOKEN_IDENT {
            throw "Expected return type"
        }
        bump(sess)
        sess["pReturnType"] = sess["pTok"].value
    }
    return 0
}
""",
    )
    write(
        "parser_stmt_fn_where.kab",
        IMPORTS
        + """
pub fn parseStmt_fn_where(sess) {
    sess["pWhereClause"] = []
    if sess["pCur"].type != TOKEN_WHERE {
        return 0
    }
    bump(sess)
    while true {
        sess["pTok"] = sess["pCur"]
        if sess["pTok"].type != TOKEN_IDENT {
            throw "Expected type parameter in where"
        }
        bump(sess)
        sess["pWhereParam"] = sess["pTok"].value
        if sess["pCur"].type != ":" {
            throw "Expected : in where"
        }
        bump(sess)
        sess["pTok"] = sess["pCur"]
        if sess["pTok"].type != TOKEN_IDENT {
            throw "Expected trait name in where"
        }
        bump(sess)
        sess["pWhereTrait"] = sess["pTok"].value
        sess["pWhereClause"] = push(sess["pWhereClause"], {
            "typeParam": sess["pWhereParam"],
            "traitName": sess["pWhereTrait"]
        })
        if sess["pCur"].type == "," {
            bump(sess)
        } else {
            break
        }
    }
    return 0
}
""",
    )
    write(
        "parser_stmt_fn_body.kab",
        IMPORTS
        + """
pub fn parseStmt_fn_body(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == "EOF" {
            throw "Unclosed block"
        }
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    let fnBody = sess["pBody"]
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return {
        "kind": AST_FN,
        "sym": sess["pFnSym"],
        "isPub": sess["pFnPub"],
        "typeParams": sess["pTypeParams"],
        "params": sess["pParams"],
        "paramTypes": sess["pParamTypes"],
        "returnType": sess["pReturnType"],
        "whereClause": sess["pWhereClause"],
        "body": { "kind": AST_BLOCK, "body": fnBody }
    }
}
""",
    )
    write(
        "parser_stmt_fn.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_fn_sig"
import "self_host/parser_stmt_fn_where"
import "self_host/parser_stmt_fn_body"

pub fn parseStmt_fn(sess) {
    if !(sess["pCur"].type == TOKEN_FN) {
        return null
    }
    parseStmt_fn_sig(sess)
    parseStmt_fn_where(sess)
    return parseStmt_fn_body(sess)
}
""",
    )


def split_stmt_iface() -> None:
    if already("parser_stmt_iface.kab", "parser_stmt_iface_meth"):
        print("skip parser_stmt_iface")
        return
    write(
        "parser_stmt_iface_assoc.kab",
        IMPORTS
        + """
pub fn parseStmt_iface_assoc(sess) {
    if !(sess["pCur"].type == TOKEN_IDENT && sess["pCur"].value == "type") {
        return 0
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected associated type name"
    }
    bump(sess)
    sess["pAssocTypes"] = push(sess["pAssocTypes"], sess["pTok"].value)
    if sess["pCur"].type != ";" {
        throw "Expected ; after associated type"
    }
    bump(sess)
    return 1
}
""",
    )
    write(
        "parser_stmt_iface_meth.kab",
        IMPORTS
        + """
pub fn parseStmt_iface_meth(sess) {
    if sess["pCur"].type != TOKEN_FN {
        throw "Expected fn in interface"
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected method name"
    }
    bump(sess)
    sess["pFnSym"] = sess["pTok"].value
    if sess["pCur"].type != "(" {
        throw "Expected ( at " + ("" + sess["pCur"].line) + ":" + ("" + sess["pCur"].column) + " tok=" + sess["pCur"].type
    }
    bump(sess)
    sess["pMethParams"] = []
    if sess["pCur"].type != ")" {
        while true {
            sess["pTok"] = sess["pCur"]
            if sess["pTok"].type != TOKEN_IDENT {
                throw "Expected parameter name"
            }
            bump(sess)
            sess["pMethParams"] = push(sess["pMethParams"], poolPush(sess, sess["pTok"].value))
            if sess["pCur"].type == "," {
                bump(sess)
            } else {
                break
            }
        }
    }
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    let ifaceMethParams = sess["pMethParams"]
    let ifaceMethSym = sess["pFnSym"]
    if sess["pCur"].type == "{" {
        bump(sess)
        sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
        sess["pBodyDepth"] = sess["pBodyDepth"] + 1
        sess["pBody"] = []
        while sess["pCur"].type != "}" {
            if sess["pCur"].type == "EOF" {
                throw "Unclosed default method body"
            }
            sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
            if sess["pCur"].type == ";" {
                bump(sess)
            }
        }
        bump(sess)
        let defBody = sess["pBody"]
        sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
        sess["pBodyStack"] = pop(sess["pBodyStack"])
        sess["pBodyDepth"] = sess["pBodyDepth"] - 1
        sess["pIfaceMethods"] = push(sess["pIfaceMethods"], {
            "sym": ifaceMethSym,
            "params": ifaceMethParams,
            "body": { "kind": AST_BLOCK, "body": defBody }
        })
    } else {
        if sess["pCur"].type != ";" {
            throw "Expected ; or default method body"
        }
        bump(sess)
        sess["pIfaceMethods"] = push(sess["pIfaceMethods"], { "sym": ifaceMethSym, "params": ifaceMethParams })
    }
    return 0
}
""",
    )
    write(
        "parser_stmt_iface.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_iface_assoc"
import "self_host/parser_stmt_iface_meth"

pub fn parseStmt_iface(sess) {
    if !(sess["pCur"].type == TOKEN_INTERFACE || sess["pCur"].type == TOKEN_TRAIT) {
        return null
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected interface/trait name"
    }
    bump(sess)
    sess["pIfaceSym"] = sess["pTok"].value
    sess["pTypeParams"] = pCallTypeArgs(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pIfaceMethods"] = []
    sess["pAssocTypes"] = []
    while sess["pCur"].type != "}" {
        let gotAssoc = parseStmt_iface_assoc(sess)
        if gotAssoc == 0 {
            parseStmt_iface_meth(sess)
        }
    }
    bump(sess)
    return {
        "kind": AST_INTERFACE,
        "sym": sess["pIfaceSym"],
        "typeParams": sess["pTypeParams"],
        "methods": sess["pIfaceMethods"],
        "associatedTypes": sess["pAssocTypes"]
    }
}
""",
    )


def split_compare_logic() -> None:
    if already("parser_compare_logic.kab", "parser_compare_logic_and"):
        print("skip parser_compare_logic")
        return
    write(
        "parser_compare_logic_and.kab",
        IMPORTS
        + """
pub fn parseCompare_logic_and(sess) {
    while sess["pCur"].type == "&&" {
        bump(sess)
        sess["pExprLeft"] = sess["pLeft"]
        sess["pRight"] = pCallRelExpr(sess)
        if sess["pNoBit"] == 0 {
            while sess["pCur"].type == "&" || sess["pCur"].type == "^" || sess["pCur"].type == "|" {
                sess["pBinOp"] = sess["pCur"].type
                bump(sess)
                sess["pRight"] = { "kind": AST_BINARY, "op": sess["pBinOp"], "left": sess["pRight"], "right": pCallRelExpr(sess) }
            }
        }
        sess["pLeft"] = { "kind": AST_BINARY, "op": "&&", "left": sess["pExprLeft"], "right": sess["pRight"] }
    }
    return 0
}
""",
    )
    write(
        "parser_compare_logic_or.kab",
        IMPORTS
        + """
pub fn parseCompare_logic_or(sess) {
    while sess["pCur"].type == "||" {
        bump(sess)
        sess["pExprLeft"] = sess["pLeft"]
        sess["pRight"] = pCallRelExpr(sess)
        if sess["pNoBit"] == 0 {
            while sess["pCur"].type == "&" || sess["pCur"].type == "^" || sess["pCur"].type == "|" {
                sess["pBinOp"] = sess["pCur"].type
                bump(sess)
                sess["pRight"] = { "kind": AST_BINARY, "op": sess["pBinOp"], "left": sess["pRight"], "right": pCallRelExpr(sess) }
            }
        }
        while sess["pCur"].type == "&&" {
            bump(sess)
            let andLeft = sess["pRight"]
            let andRight = pCallRelExpr(sess)
            if sess["pNoBit"] == 0 {
                while sess["pCur"].type == "&" || sess["pCur"].type == "^" || sess["pCur"].type == "|" {
                    sess["pBinOp"] = sess["pCur"].type
                    bump(sess)
                    andRight = { "kind": AST_BINARY, "op": sess["pBinOp"], "left": andRight, "right": pCallRelExpr(sess) }
                }
            }
            sess["pRight"] = { "kind": AST_BINARY, "op": "&&", "left": andLeft, "right": andRight }
        }
        sess["pLeft"] = { "kind": AST_BINARY, "op": "||", "left": sess["pExprLeft"], "right": sess["pRight"] }
    }
    return 0
}
""",
    )
    write(
        "parser_compare_logic.kab",
        IMPORTS
        + """
import "self_host/parser_compare_logic_and"
import "self_host/parser_compare_logic_or"

pub fn parseCompare_logic(sess) {
    parseCompare_logic_and(sess)
    parseCompare_logic_or(sess)
    return 0
}
""",
    )


def split_postfix_tail() -> None:
    if already("parser_postfix_tail.kab", "parser_postfix_tail_call"):
        print("skip parser_postfix_tail")
        return
    write(
        "parser_postfix_tail_generic.kab",
        IMPORTS
        + """
pub fn parsePostfix_tail_generic(sess) {
    if sess["pCur"].type != "<" {
        return 0
    }
    sess["pIsGenericCallee"] = 0
    if sess["pLeft"].kind == AST_VAR {
        sess["pIsGenericCallee"] = 1
    }
    if sess["pLeft"].kind == AST_MEMBER {
        sess["pIsGenericCallee"] = 1
    }
    if sess["pIsGenericCallee"] == 1 {
        sess["pSnapTypeArgs"] = pCallTypeArgs(sess)
        if len(sess["pSnapTypeArgs"]) > 0 {
            sess["pTypeArgs"] = sess["pSnapTypeArgs"]
            return 1
        }
    }
    return -1
}
""",
    )
    write(
        "parser_postfix_tail_call.kab",
        IMPORTS
        + """
pub fn parsePostfix_tail_call(sess) {
    if sess["pCur"].type != "(" {
        return 0
    }
    sess["pCallee"] = sess["pLeft"]
    if sess["pCallee"].kind == AST_VAR {
        sess["pCallee"] = { "kind": AST_VAR, "sym": sess["pCallee"]["sym"] }
    }
    let savedCallee = sess["pCallee"]
    let savedTypeArgs = sess["pTypeArgs"]
    bump(sess)
    sess["pArgs"] = []
    if sess["pCur"].type != ")" {
        while true {
            sess["pArgs"] = push(sess["pArgs"], pCallCompare(sess))
            if sess["pCur"].type == "," {
                bump(sess)
            } else {
                break
            }
        }
    }
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    sess["pLeft"] = { "kind": AST_CALL, "callee": savedCallee, "args": sess["pArgs"], "typeArgs": savedTypeArgs }
    sess["pTypeArgs"] = []
    return 1
}
""",
    )
    write(
        "parser_postfix_tail_member.kab",
        IMPORTS
        + """
pub fn parsePostfix_tail_member(sess) {
    if sess["pCur"].type != "." {
        return 0
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    sess["pMemberName"] = ""
    if sess["pTok"].type == TOKEN_IDENT {
        bump(sess)
        sess["pMemberName"] = sess["pTok"].value
    } else {
        if sess["pTok"].type == TOKEN_NONE {
            bump(sess)
            sess["pMemberName"] = "None"
        } else {
            if sess["pTok"].type == TOKEN_SOME {
                bump(sess)
                sess["pMemberName"] = "Some"
            } else {
                if sess["pTok"].type == TOKEN_OK {
                    bump(sess)
                    sess["pMemberName"] = "Ok"
                } else {
                    if sess["pTok"].type == TOKEN_ERR {
                        bump(sess)
                        sess["pMemberName"] = "Err"
                    } else {
                        throw "Expected member name"
                    }
                }
            }
        }
    }
    sess["pMemberName"] = poolPush(sess, sess["pMemberName"])
    sess["pLeft"] = {
        "kind": AST_MEMBER,
        "object": sess["pLeft"],
        "field": sess["pMemberName"],
        "typeArgs": sess["pTypeArgs"]
    }
    sess["pTypeArgs"] = []
    return 1
}
""",
    )
    write(
        "parser_postfix_tail_index.kab",
        IMPORTS
        + """
pub fn parsePostfix_tail_index(sess) {
    if sess["pCur"].type != "[" {
        return 0
    }
    bump(sess)
    let indexObj = sess["pLeft"]
    sess["pRight"] = pCallCompare(sess)
    if sess["pCur"].type != "]" {
        throw "Expected ]"
    }
    bump(sess)
    sess["pLeft"] = { "kind": AST_INDEX, "object": indexObj, "index": sess["pRight"] }
    return 1
}
""",
    )
    write(
        "parser_postfix_tail.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_tail_generic"
import "self_host/parser_postfix_tail_call"
import "self_host/parser_postfix_tail_member"
import "self_host/parser_postfix_tail_index"

pub fn parsePostfix_tail(sess) {
    while true {
        let g = parsePostfix_tail_generic(sess)
        if g == 1 {
            // continue loop for call/member after type args
        } else {
            if g == -1 {
                break
            }
            let c = parsePostfix_tail_call(sess)
            if c == 0 {
                let m = parsePostfix_tail_member(sess)
                if m == 0 {
                    let ix = parsePostfix_tail_index(sess)
                    if ix == 0 {
                        break
                    }
                }
            }
        }
    }
    return sess["pLeft"]
}
""",
    )


def split_type_args() -> None:
    if already("parser_type_args.kab", "parser_type_args_scan"):
        print("skip parser_type_args")
        return
    write(
        "parser_type_args_scan.kab",
        IMPORTS
        + """
pub fn parseTypeArgs_scan(sess) {
    if sess["pCur"].type != "<" {
        return 0
    }
    sess["pAngleI"] = sess["pPos"] + 1
    if sess["pAngleI"] >= sess["pToksLen"] {
        return 0
    }
    sess["pAngleTok"] = sess["pToks"][sess["pAngleI"]]
    if sess["pAngleTok"].type != TOKEN_IDENT {
        return 0
    }
    sess["pAngleI"] = sess["pAngleI"] + 1
    while sess["pAngleI"] < sess["pToksLen"] {
        sess["pAngleTok"] = sess["pToks"][sess["pAngleI"]]
        if sess["pAngleTok"].type == ">" {
            break
        }
        if sess["pAngleTok"].type == "<" {
            return 0
        }
        if sess["pAngleTok"].type == "," {
            sess["pAngleI"] = sess["pAngleI"] + 1
        } else {
            if sess["pAngleTok"].type == TOKEN_IDENT {
                sess["pAngleI"] = sess["pAngleI"] + 1
            } else {
                return 0
            }
        }
    }
    if sess["pAngleI"] >= sess["pToksLen"] {
        return 0
    }
    if sess["pToks"][sess["pAngleI"]].type != ">" {
        return 0
    }
    return 1
}
""",
    )
    write(
        "parser_type_args_consume.kab",
        IMPORTS
        + """
pub fn parseTypeArgs_consume(sess) {
    bump(sess)
    sess["pAngleOut"] = []
    while sess["pCur"].type != ">" {
        sess["pTok"] = sess["pCur"]
        if sess["pTok"].type != TOKEN_IDENT {
            throw "Expected type name"
        }
        bump(sess)
        sess["pAngleOut"] = push(sess["pAngleOut"], poolPush(sess, sess["pTok"].value))
        if sess["pCur"].type == "," {
            bump(sess)
        }
    }
    if sess["pCur"].type != ">" {
        throw "Expected >"
    }
    bump(sess)
    return sess["pAngleOut"]
}
""",
    )
    write(
        "parser_type_args.kab",
        IMPORTS
        + """
import "self_host/parser_type_args_scan"
import "self_host/parser_type_args_consume"

pub fn parseTypeArgs(sess) {
    if parseTypeArgs_scan(sess) == 0 {
        return []
    }
    return parseTypeArgs_consume(sess)
}
""",
    )


def split_main() -> None:
    if already("parser_main.kab", "parser_main_init"):
        print("skip parser_main")
        return
    write(
        "parser_main_init.kab",
        IMPORTS
        + """
pub fn parseMain_init(sess, tokens) {
    sess["pToks"] = tokens
    sess["pToksLen"] = len(tokens)
    sess["pPos"] = 0
    if sess["pToksLen"] > 0 {
        sess["pCur"] = sess["pToks"][0]
    } else {
        sess["pCur"] = sess["pEofTok"]
    }
    sess["pSymPool"] = []
    sess["pSymN"] = 0
    sess["pImports"] = []
    sess["pCondStack"] = []
    sess["pCondDepth"] = 0
    sess["pAddLeftStack"] = []
    sess["pBodyStack"] = []
    sess["pBodyDepth"] = 0
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = 1
    sess["pBody"] = []
    sess["pDone"] = 0
    return 0
}
""",
    )
    write(
        "parser_main_loop.kab",
        IMPORTS
        + """
pub fn parseMain_loop(sess) {
    while sess["pDone"] == 0 {
        if sess["pPos"] >= sess["pToksLen"] {
            sess["pDone"] = 1
        }
        if sess["pDone"] == 0 {
            sess["pTok"] = sess["pCur"]
            if sess["pTok"].type == "EOF" {
                sess["pDone"] = 1
            }
        }
        if sess["pDone"] == 0 {
            if sess["pTok"].type == TOKEN_IMPORT {
                bump(sess)
                sess["pTok"] = sess["pCur"]
                if sess["pTok"].type == TOKEN_STRING {
                    bump(sess)
                    sess["pImports"] = push(sess["pImports"], sess["pTok"].value)
                }
                if sess["pCur"].type == ";" {
                    bump(sess)
                }
            } else {
                sess["pVal"] = pCallStmt(sess)
                if sess["pVal"] != null {
                    sess["pBody"] = push(sess["pBody"], sess["pVal"])
                }
            }
            if sess["pCur"].type == ";" {
                bump(sess)
            }
        }
    }
    return 0
}
""",
    )
    write(
        "parser_main.kab",
        IMPORTS
        + """
import "self_host/parser_main_init"
import "self_host/parser_main_loop"

pub fn parseMain(sess, tokens) {
    parseMain_init(sess, tokens)
    parseMain_loop(sess)
    let programBody = sess["pBody"]
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return { "kind": AST_PROGRAM, "body": programBody, "imports": sess["pImports"] }
}
""",
    )


def split_hooks() -> None:
    if already("parser_hooks.kab", "parser_hooks_expr"):
        print("skip parser_hooks")
        return
    write(
        "parser_hooks_expr.kab",
        """// P6b: parser trampoline hooks (expr).
pub fn pCallPostfix(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 0
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallTypeArgs(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 1
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallUnary(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 2
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallMul(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 3
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}
""",
    )
    write(
        "parser_hooks_stmt.kab",
        """// P6b: parser trampoline hooks (stmt/compare).
pub fn pCallAddShift(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 4
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallRelExpr(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 5
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallCompare(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 6
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallStmt(sess) {
    let prevH = sess["_hook"]
    sess["_hook"] = 7
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}
""",
    )
    write(
        "parser_hooks.kab",
        """// P6b: parser trampoline hooks.
import "self_host/parser_hooks_expr"
import "self_host/parser_hooks_stmt"
""",
    )


def split_stmt_enum() -> None:
    if already("parser_stmt_enum.kab", "parser_stmt_enum_variant"):
        print("skip parser_stmt_enum")
        return
    write(
        "parser_stmt_enum_name.kab",
        IMPORTS
        + """
pub fn parseStmt_enum_name(sess) {
    sess["pTok"] = sess["pCur"]
    sess["pVariantName"] = ""
    if sess["pTok"].type == TOKEN_IDENT {
        bump(sess)
        sess["pVariantName"] = sess["pTok"].value
        return 0
    }
    if sess["pTok"].type == TOKEN_NONE {
        bump(sess)
        sess["pVariantName"] = "None"
        return 0
    }
    if sess["pTok"].type == TOKEN_SOME {
        bump(sess)
        sess["pVariantName"] = "Some"
        return 0
    }
    if sess["pTok"].type == TOKEN_OK {
        bump(sess)
        sess["pVariantName"] = "Ok"
        return 0
    }
    if sess["pTok"].type == TOKEN_ERR {
        bump(sess)
        sess["pVariantName"] = "Err"
        return 0
    }
    if sess["pTok"].type == TOKEN_TRUE {
        bump(sess)
        sess["pVariantName"] = "True"
        return 0
    }
    if sess["pTok"].type == TOKEN_FALSE {
        bump(sess)
        sess["pVariantName"] = "False"
        return 0
    }
    throw "Expected variant name"
}
""",
    )
    write(
        "parser_stmt_enum_variant.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_enum_name"

pub fn parseStmt_enum_variant(sess) {
    parseStmt_enum_name(sess)
    sess["pVariantSym"] = poolPush(sess, sess["pVariantName"])
    sess["pVariantFields"] = []
    if sess["pCur"].type == "(" {
        bump(sess)
        while sess["pCur"].type != ")" {
            sess["pTok"] = sess["pCur"]
            if sess["pTok"].type != TOKEN_IDENT {
                throw "Expected variant field type"
            }
            bump(sess)
            sess["pVariantFields"] = push(sess["pVariantFields"], poolPush(sess, sess["pTok"].value))
            if sess["pCur"].type == "," {
                bump(sess)
            }
        }
        bump(sess)
    }
    sess["pVariants"] = push(sess["pVariants"], {
        "name": sess["pVariantSym"],
        "fields": sess["pVariantFields"]
    })
    if sess["pCur"].type == "," {
        bump(sess)
    }
    return 0
}
""",
    )
    write(
        "parser_stmt_enum.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_enum_variant"

pub fn parseStmt_enum(sess) {
    if !(sess["pCur"].type == TOKEN_ENUM) {
        return null
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected enum name"
    }
    bump(sess)
    sess["pEnumSym"] = sess["pTok"].value
    sess["pTypeParams"] = pCallTypeArgs(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pVariants"] = []
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == "," {
            bump(sess)
        } else {
            parseStmt_enum_variant(sess)
        }
    }
    bump(sess)
    return { "kind": AST_ENUM, "sym": sess["pEnumSym"], "typeParams": sess["pTypeParams"], "variants": sess["pVariants"] }
}
""",
    )


def split_stmt_try_if() -> None:
    if already("parser_stmt_try.kab", "parser_stmt_try_catch"):
        print("skip parser_stmt_try")
    else:
        write(
            "parser_stmt_try_body.kab",
            IMPORTS
            + """
pub fn parseStmt_try_body(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == "EOF" {
            throw "Unclosed try"
        }
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    let tryBody = sess["pBody"]
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return tryBody
}
""",
        )
        write(
            "parser_stmt_try_catch.kab",
            IMPORTS
            + """
pub fn parseStmt_try_catch(sess) {
    if sess["pCur"].type != TOKEN_CATCH {
        throw "Expected catch"
    }
    bump(sess)
    if sess["pCur"].type != "(" {
        throw "Expected ( at " + ("" + sess["pCur"].line) + ":" + ("" + sess["pCur"].column) + " tok=" + sess["pCur"].type
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected catch binding"
    }
    bump(sess)
    let errSym = sess["pTok"].value
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == "EOF" {
            throw "Unclosed catch"
        }
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    let catchBody = sess["pBody"]
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return {
        "errName": errSym,
        "handler": { "kind": AST_BLOCK, "body": catchBody }
    }
}
""",
        )
        write(
            "parser_stmt_try.kab",
            IMPORTS
            + """
import "self_host/parser_stmt_try_body"
import "self_host/parser_stmt_try_catch"

pub fn parseStmt_try(sess) {
    if !(sess["pCur"].type == TOKEN_TRY) {
        return null
    }
    bump(sess)
    let tryBody = parseStmt_try_body(sess)
    let catchPart = parseStmt_try_catch(sess)
    return {
        "kind": AST_TRY,
        "body": { "kind": AST_BLOCK, "body": tryBody },
        "errName": catchPart["errName"],
        "handler": catchPart["handler"]
    }
}
""",
        )

    if already("parser_stmt_if.kab", "parser_stmt_if_arm"):
        print("skip parser_stmt_if")
        return
    write(
        "parser_stmt_if_arm.kab",
        IMPORTS
        + """
pub fn parseStmt_if_arm(sess) {
    let armCond = pCallCompare(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    while sess["pCur"].type != "}" {
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    let armThen = { "kind": AST_BLOCK, "body": sess["pBody"] }
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return { "cond": armCond, "then": armThen }
}
""",
    )
    write(
        "parser_stmt_if_else_block.kab",
        IMPORTS
        + """
pub fn parseStmt_if_else_block(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    while sess["pCur"].type != "}" {
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    let finalElse = { "kind": AST_BLOCK, "body": sess["pBody"] }
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return finalElse
}
""",
    )
    write(
        "parser_stmt_if.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_if_arm"
import "self_host/parser_stmt_if_else_block"

pub fn parseStmt_if(sess) {
    if !(sess["pCur"].type == TOKEN_IF) {
        return null
    }
    bump(sess)
    // Iterative else-if: avoid deep parseStmt recursion (hangs on long chains).
    let chainConds = []
    let chainThens = []
    let finalElse = null
    while true {
        let arm = parseStmt_if_arm(sess)
        chainConds = push(chainConds, arm["cond"])
        chainThens = push(chainThens, arm["then"])
        if sess["pCur"].type != TOKEN_ELSE {
            break
        }
        bump(sess)
        if sess["pCur"].type == TOKEN_IF {
            bump(sess)
            continue
        }
        finalElse = parseStmt_if_else_block(sess)
        break
    }
    let i = len(chainConds) - 1
    let ifNode = { "kind": AST_IF, "cond": chainConds[i], "then": chainThens[i], "elseBranch": finalElse }
    i = i - 1
    while i >= 0 {
        ifNode = { "kind": AST_IF, "cond": chainConds[i], "then": chainThens[i], "elseBranch": ifNode }
        i = i - 1
    }
    return ifNode
}
""",
    )


def main() -> None:
    split_session()
    split_stmt_class()
    split_postfix_lit()
    split_stmt_fn()
    split_stmt_iface()
    split_compare_logic()
    split_postfix_tail()
    split_type_args()
    split_main()
    # hooks: do NOT facade-split — Kabootar does not re-export via import-only modules
    split_stmt_enum()
    split_stmt_try_if()
    print("done — run test_parser.kab and _parser_all_shard_times.py")


if __name__ == "__main__":
    main()
