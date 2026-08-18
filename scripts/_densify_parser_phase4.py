#!/usr/bin/env python3
"""P6b phase 4: densify remaining >10s parser shards.

Run from repo root (idempotent via markers):
  python scripts/_densify_parser_phase4.py
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

BLOCK_IMPORTS = IMPORTS + 'import "self_host/parser_block"\n'


def write(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print(f"wrote {p.relative_to(ROOT)} ({len(text.splitlines())} lines)")


def already(name: str, marker: str) -> bool:
    p = SH / name
    return p.is_file() and marker in p.read_text(encoding="utf-8")


def write_block_helpers() -> None:
    if already("parser_block.kab", "pEnterBody"):
        print("skip parser_block")
        return
    write(
        "parser_block.kab",
        IMPORTS
        + """
pub fn pEnterBody(sess) {
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pBody"] = []
    return 0
}

pub fn pLeaveBody(sess) {
    let body = sess["pBody"]
    sess["pBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return body
}

pub fn pParseBodyUntilRBrace(sess, eofMsg) {
    while sess["pCur"].type != "}" {
        if sess["pCur"].type == "EOF" {
            throw eofMsg
        }
        sess["pBody"] = push(sess["pBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    return 0
}

pub fn pEnterLoopBody(sess) {
    sess["pBodyStack"] = push(sess["pBodyStack"], sess["pLoopBody"])
    sess["pBodyDepth"] = sess["pBodyDepth"] + 1
    sess["pLoopBody"] = []
    return 0
}

pub fn pLeaveLoopBody(sess) {
    let body = sess["pLoopBody"]
    sess["pLoopBody"] = sess["pBodyStack"][sess["pBodyDepth"] - 1]
    sess["pBodyStack"] = pop(sess["pBodyStack"])
    sess["pBodyDepth"] = sess["pBodyDepth"] - 1
    return body
}

pub fn pParseLoopUntilRBrace(sess) {
    while sess["pCur"].type != "}" {
        sess["pLoopBody"] = push(sess["pLoopBody"], pCallStmt(sess))
        if sess["pCur"].type == ";" {
            bump(sess)
        }
    }
    bump(sess)
    return 0
}
""",
    )


def split_hooks() -> None:
    if already("parser_hooks.kab", "pCallHook"):
        print("skip parser_hooks")
        return
    write(
        "parser_hooks.kab",
        """// P6b: parser trampoline hooks (shared AccAdd path).
pub fn pCallHook(sess, n) {
    let prevH = sess["_hook"]
    sess["_hook"] = n
    let r = sess["tramp"]()
    sess["_hook"] = prevH
    return r
}

pub fn pCallPostfix(sess) {
    return pCallHook(sess, 0)
}

pub fn pCallTypeArgs(sess) {
    return pCallHook(sess, 1)
}

pub fn pCallUnary(sess) {
    return pCallHook(sess, 2)
}

pub fn pCallMul(sess) {
    return pCallHook(sess, 3)
}

pub fn pCallAddShift(sess) {
    return pCallHook(sess, 4)
}

pub fn pCallRelExpr(sess) {
    return pCallHook(sess, 5)
}

pub fn pCallCompare(sess) {
    return pCallHook(sess, 6)
}

pub fn pCallStmt(sess) {
    return pCallHook(sess, 7)
}
""",
    )


def split_class_method() -> None:
    if already("parser_stmt_class_method.kab", "parser_stmt_class_method_params"):
        print("skip parser_stmt_class_method")
        return
    write(
        "parser_stmt_class_method_params.kab",
        IMPORTS
        + """
pub fn parseStmt_class_method_params(sess) {
    sess["pMethParams"] = []
    sess["pMethParamTypes"] = []
    if sess["pCur"].type == ")" {
        return 0
    }
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
    return 0
}
""",
    )
    write(
        "parser_stmt_class_method_body.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_class_method_body(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterBody(sess)
    pParseBodyUntilRBrace(sess, "Unclosed block")
    let methBody = pLeaveBody(sess)
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
        "parser_stmt_class_method.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_class_method_params"
import "self_host/parser_stmt_class_method_body"

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
    parseStmt_class_method_params(sess)
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    return parseStmt_class_method_body(sess)
}
""",
    )


def split_postfix_paren() -> None:
    if already("parser_postfix_paren.kab", "parser_postfix_paren_arrow"):
        print("skip parser_postfix_paren")
        return
    write(
        "parser_postfix_paren_arrow.kab",
        BLOCK_IMPORTS
        + """
pub fn parsePostfix_paren_arrow(sess, arrowParams) {
    bump(sess)
    let arrowBody = null
    if sess["pCur"].type == "{" {
        bump(sess)
        pEnterBody(sess)
        pParseBodyUntilRBrace(sess, "Unclosed arrow block")
        let ab = pLeaveBody(sess)
        arrowBody = { "kind": AST_BLOCK, "body": ab }
    } else {
        arrowBody = pCallCompare(sess)
    }
    sess["pLeft"] = { "kind": AST_ARROW, "params": arrowParams, "body": arrowBody }
    return 0
}
""",
    )
    write(
        "parser_postfix_paren_scan.kab",
        IMPORTS
        + """
pub fn parsePostfix_paren_scan(sess) {
    let arrowParams = []
    let arrowOk = 1
    if sess["pCur"].type == ")" {
        bump(sess)
        return { "ok": arrowOk, "params": arrowParams }
    }
    while sess["pCur"].type != ")" {
        if sess["pCur"].type != TOKEN_IDENT {
            arrowOk = 0
            break
        }
        sess["pTok"] = sess["pCur"]
        bump(sess)
        arrowParams = push(arrowParams, sess["pTok"].value)
        if sess["pCur"].type == "," {
            bump(sess)
        } else {
            if sess["pCur"].type != ")" {
                arrowOk = 0
                break
            }
        }
    }
    if arrowOk == 1 {
        if sess["pCur"].type != ")" {
            arrowOk = 0
        } else {
            bump(sess)
        }
    }
    return { "ok": arrowOk, "params": arrowParams }
}
""",
    )
    write(
        "parser_postfix_paren_group.kab",
        IMPORTS
        + """
pub fn parsePostfix_paren_group(sess, savePos) {
    sess["pPos"] = savePos
    if sess["pPos"] < sess["pToksLen"] {
        sess["pCur"] = sess["pToks"][sess["pPos"]]
    } else {
        sess["pCur"] = sess["pEofTok"]
    }
    bump(sess)
    sess["pLeft"] = pCallCompare(sess)
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    return 0
}
""",
    )
    write(
        "parser_postfix_paren.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_paren_scan"
import "self_host/parser_postfix_paren_arrow"
import "self_host/parser_postfix_paren_group"

pub fn parsePostfix_paren(sess) {
    if sess["pLeft"] != null {
        return 0
    }
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != "(" {
        return 0
    }
    let savePos = sess["pPos"]
    bump(sess)
    let scanned = parsePostfix_paren_scan(sess)
    if scanned["ok"] == 1 && sess["pCur"].type == "=>" {
        parsePostfix_paren_arrow(sess, scanned["params"])
    } else {
        parsePostfix_paren_group(sess, savePos)
    }
    return 0
}
""",
    )


def split_iface_meth() -> None:
    if already("parser_stmt_iface_meth.kab", "parser_stmt_iface_meth_sig"):
        print("skip parser_stmt_iface_meth")
        return
    write(
        "parser_stmt_iface_meth_sig.kab",
        IMPORTS
        + """
pub fn parseStmt_iface_meth_sig(sess) {
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
    return 0
}
""",
    )
    write(
        "parser_stmt_iface_meth_tail.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_iface_meth_tail(sess) {
    let ifaceMethParams = sess["pMethParams"]
    let ifaceMethSym = sess["pFnSym"]
    if sess["pCur"].type == "{" {
        bump(sess)
        pEnterBody(sess)
        pParseBodyUntilRBrace(sess, "Unclosed default method body")
        let defBody = pLeaveBody(sess)
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
        "parser_stmt_iface_meth.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_iface_meth_sig"
import "self_host/parser_stmt_iface_meth_tail"

pub fn parseStmt_iface_meth(sess) {
    if sess["pCur"].type != TOKEN_FN {
        throw "Expected fn in interface"
    }
    parseStmt_iface_meth_sig(sess)
    return parseStmt_iface_meth_tail(sess)
}
""",
    )


def split_obj_arr() -> None:
    if already("parser_postfix_obj_arr.kab", "parser_postfix_obj"):
        print("skip parser_postfix_obj_arr")
        return
    write(
        "parser_postfix_obj.kab",
        IMPORTS
        + """
pub fn parsePostfix_obj(sess) {
    if sess["pLeft"] != null {
        return 0
    }
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != "{" {
        return 0
    }
    bump(sess)
    sess["pObjFields"] = []
    while sess["pCur"].type != "}" {
        sess["pTok"] = sess["pCur"]
        if sess["pTok"].type == TOKEN_IDENT {
            bump(sess)
            sess["pSaveSym"] = poolPush(sess, sess["pTok"].value)
        } else {
            if sess["pTok"].type == TOKEN_STRING {
                bump(sess)
                sess["pSaveSym"] = poolPush(sess, sess["pTok"].value)
            } else {
                throw "Expected object key"
            }
        }
        if sess["pCur"].type != ":" {
            throw "Expected :"
        }
        bump(sess)
        sess["pVal"] = pCallCompare(sess)
        sess["pObjFields"] = push(sess["pObjFields"], { "key": sess["pSaveSym"], "value": sess["pVal"] })
        if sess["pCur"].type == "," {
            bump(sess)
        }
    }
    bump(sess)
    sess["pLeft"] = { "kind": AST_OBJECT, "fields": sess["pObjFields"] }
    return 0
}
""",
    )
    write(
        "parser_postfix_arr.kab",
        IMPORTS
        + """
pub fn parsePostfix_arr(sess) {
    if sess["pLeft"] != null {
        return 0
    }
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != "[" {
        return 0
    }
    bump(sess)
    sess["pArgs"] = []
    while sess["pCur"].type != "]" {
        sess["pArgs"] = push(sess["pArgs"], pCallCompare(sess))
        if sess["pCur"].type == "," {
            bump(sess)
        }
    }
    bump(sess)
    sess["pLeft"] = { "kind": AST_ARRAY, "elems": sess["pArgs"] }
    return 0
}
""",
    )
    write(
        "parser_postfix_obj_arr.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_obj"
import "self_host/parser_postfix_arr"

pub fn parsePostfix_obj_arr(sess) {
    if sess["pLeft"] != null {
        return 0
    }
    parsePostfix_obj(sess)
    parsePostfix_arr(sess)
    return 0
}
""",
    )


def split_fn_sig() -> None:
    if already("parser_stmt_fn_sig.kab", "parser_stmt_fn_params"):
        print("skip parser_stmt_fn_sig")
        return
    write(
        "parser_stmt_fn_params.kab",
        IMPORTS
        + """
pub fn parseStmt_fn_params(sess) {
    sess["pParams"] = []
    sess["pParamTypes"] = []
    if sess["pCur"].type == ")" {
        return 0
    }
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
    return 0
}
""",
    )
    write(
        "parser_stmt_fn_ret.kab",
        IMPORTS
        + """
pub fn parseStmt_fn_ret(sess) {
    sess["pReturnType"] = ""
    if sess["pCur"].type != "->" {
        return 0
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected return type"
    }
    bump(sess)
    sess["pReturnType"] = sess["pTok"].value
    return 0
}
""",
    )
    write(
        "parser_stmt_fn_sig.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_fn_params"
import "self_host/parser_stmt_fn_ret"

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
    parseStmt_fn_params(sess)
    if sess["pCur"].type != ")" {
        throw "Expected )"
    }
    bump(sess)
    return parseStmt_fn_ret(sess)
}
""",
    )


def split_logic_or() -> None:
    if already("parser_compare_logic_or.kab", "parser_compare_logic_or_bit"):
        print("skip parser_compare_logic_or")
        return
    write(
        "parser_compare_logic_or_bit.kab",
        IMPORTS
        + """
pub fn parseCompare_logic_or_bit(sess) {
    if sess["pNoBit"] != 0 {
        return 0
    }
    while sess["pCur"].type == "&" || sess["pCur"].type == "^" || sess["pCur"].type == "|" {
        sess["pBinOp"] = sess["pCur"].type
        bump(sess)
        sess["pRight"] = { "kind": AST_BINARY, "op": sess["pBinOp"], "left": sess["pRight"], "right": pCallRelExpr(sess) }
    }
    return 0
}
""",
    )
    write(
        "parser_compare_logic_or_and.kab",
        IMPORTS
        + """
import "self_host/parser_compare_logic_or_bit"

pub fn parseCompare_logic_or_and(sess) {
    while sess["pCur"].type == "&&" {
        bump(sess)
        let andLeft = sess["pRight"]
        let andRight = pCallRelExpr(sess)
        sess["pRight"] = andRight
        parseCompare_logic_or_bit(sess)
        andRight = sess["pRight"]
        sess["pRight"] = { "kind": AST_BINARY, "op": "&&", "left": andLeft, "right": andRight }
    }
    return 0
}
""",
    )
    write(
        "parser_compare_logic_or.kab",
        IMPORTS
        + """
import "self_host/parser_compare_logic_or_bit"
import "self_host/parser_compare_logic_or_and"

pub fn parseCompare_logic_or(sess) {
    while sess["pCur"].type == "||" {
        bump(sess)
        sess["pExprLeft"] = sess["pLeft"]
        sess["pRight"] = pCallRelExpr(sess)
        parseCompare_logic_or_bit(sess)
        parseCompare_logic_or_and(sess)
        sess["pLeft"] = { "kind": AST_BINARY, "op": "||", "left": sess["pExprLeft"], "right": sess["pRight"] }
    }
    return 0
}
""",
    )


def split_lit_scalar() -> None:
    if already("parser_postfix_lit_scalar.kab", "parser_postfix_lit_num"):
        print("skip parser_postfix_lit_scalar")
        return
    write(
        "parser_postfix_lit_num.kab",
        IMPORTS
        + """
pub fn parsePostfix_lit_num(sess) {
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type == TOKEN_NUMBER {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_NUMBER, "value": sess["pTok"].value }
        return 1
    }
    if sess["pTok"].type == TOKEN_STRING {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_STRING, "value": sess["pTok"].value }
        return 1
    }
    return 0
}
""",
    )
    write(
        "parser_postfix_lit_bool.kab",
        IMPORTS
        + """
pub fn parsePostfix_lit_bool(sess) {
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type == TOKEN_TRUE {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_BOOL, "value": true }
        return 1
    }
    if sess["pTok"].type == TOKEN_FALSE {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_BOOL, "value": false }
        return 1
    }
    if sess["pTok"].type == TOKEN_NULL {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_NULL, "value": null }
        return 1
    }
    if sess["pTok"].type == TOKEN_UNDEFINED {
        bump(sess)
        sess["pLeft"] = { "kind": AST_LITERAL, "lit": LIT_UNDEF, "value": undefined }
        return 1
    }
    if sess["pTok"].type == TOKEN_NONE {
        bump(sess)
        sess["pLeft"] = { "kind": AST_VAR, "sym": "None" }
        return 1
    }
    return 0
}
""",
    )
    write(
        "parser_postfix_lit_scalar.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_lit_num"
import "self_host/parser_postfix_lit_bool"

pub fn parsePostfix_lit_scalar(sess) {
    if parsePostfix_lit_num(sess) == 1 {
        return 0
    }
    parsePostfix_lit_bool(sess)
    return 0
}
""",
    )


def rewrite_fn_body_and_arrows() -> None:
    # Use shared block helpers in remaining dense bodies.
    write(
        "parser_stmt_fn_body.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_fn_body(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterBody(sess)
    pParseBodyUntilRBrace(sess, "Unclosed block")
    let fnBody = pLeaveBody(sess)
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
        "parser_postfix_bare_arrow.kab",
        BLOCK_IMPORTS
        + """
pub fn parsePostfix_bareArrow(sess) {
    if !(sess["pLeft"].kind == AST_VAR && sess["pCur"].type == "=>") {
        return 0
    }
    bump(sess)
    let bareParams = [sess["pLeft"]["sym"]]
    let bareBody = null
    if sess["pCur"].type == "{" {
        bump(sess)
        pEnterBody(sess)
        pParseBodyUntilRBrace(sess, "Unclosed arrow block")
        let bb = pLeaveBody(sess)
        bareBody = { "kind": AST_BLOCK, "body": bb }
    } else {
        bareBody = pCallCompare(sess)
    }
    sess["pLeft"] = { "kind": AST_ARROW, "params": bareParams, "body": bareBody }
    return 0
}
""",
    )
    write(
        "parser_stmt_while.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_while(sess) {
    if !(sess["pCur"].type == TOKEN_WHILE) {
        return null
    }
    bump(sess)
    sess["pCond"] = pCallCompare(sess)
    sess["pCondStack"] = push(sess["pCondStack"], sess["pCond"])
    sess["pCondDepth"] = sess["pCondDepth"] + 1
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterLoopBody(sess)
    pParseLoopUntilRBrace(sess)
    let whileBody = pLeaveLoopBody(sess)
    sess["pSave"] = sess["pCondStack"][sess["pCondDepth"] - 1]
    sess["pCondStack"] = pop(sess["pCondStack"])
    sess["pCondDepth"] = sess["pCondDepth"] - 1
    return { "kind": AST_WHILE, "cond": sess["pSave"], "body": { "kind": AST_BLOCK, "body": whileBody } }
}
""",
    )
    write(
        "parser_stmt_for.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_for(sess) {
    if !(sess["pCur"].type == TOKEN_FOR) {
        return null
    }
    bump(sess)
    sess["pTok"] = sess["pCur"]
    if sess["pTok"].type != TOKEN_IDENT {
        throw "Expected for-of binding"
    }
    bump(sess)
    let forSym = sess["pTok"].value
    if sess["pCur"].type != TOKEN_OF {
        throw "Expected of"
    }
    bump(sess)
    let forIter = pCallCompare(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterLoopBody(sess)
    pParseLoopUntilRBrace(sess)
    let forBody = pLeaveLoopBody(sess)
    return {
        "kind": AST_FOR_OF,
        "sym": forSym,
        "iterable": forIter,
        "body": { "kind": AST_BLOCK, "body": forBody }
    }
}
""",
    )
    write(
        "parser_stmt_try_body.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_try_body(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterBody(sess)
    pParseBodyUntilRBrace(sess, "Unclosed try")
    return pLeaveBody(sess)
}
""",
    )
    write(
        "parser_stmt_try_catch.kab",
        BLOCK_IMPORTS
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
    pEnterBody(sess)
    pParseBodyUntilRBrace(sess, "Unclosed catch")
    let catchBody = pLeaveBody(sess)
    return {
        "errName": errSym,
        "handler": { "kind": AST_BLOCK, "body": catchBody }
    }
}
""",
    )
    write(
        "parser_stmt_if_arm.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_if_arm(sess) {
    let armCond = pCallCompare(sess)
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterBody(sess)
    pParseBodyUntilRBrace(sess, "Unclosed if block")
    let armThen = { "kind": AST_BLOCK, "body": pLeaveBody(sess) }
    return { "cond": armCond, "then": armThen }
}
""",
    )
    write(
        "parser_stmt_if_else_block.kab",
        BLOCK_IMPORTS
        + """
pub fn parseStmt_if_else_block(sess) {
    if sess["pCur"].type != "{" {
        throw "Expected {"
    }
    bump(sess)
    pEnterBody(sess)
    pParseBodyUntilRBrace(sess, "Unclosed else block")
    return { "kind": AST_BLOCK, "body": pLeaveBody(sess) }
}
""",
    )


def split_stmt_dispatch() -> None:
    if already("parser_stmt.kab", "parser_stmt_dispatch_decl"):
        print("skip parser_stmt dispatch")
        return
    write(
        "parser_stmt_dispatch_decl.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_let"
import "self_host/parser_stmt_earlyAssign"
import "self_host/parser_stmt_enum"
import "self_host/parser_stmt_class"
import "self_host/parser_stmt_iface"
import "self_host/parser_stmt_fn"

pub fn parseStmt_dispatch_decl(sess) {
    let r = parseStmt_let(sess)
    if r != null { return r }
    r = parseStmt_earlyAssign(sess)
    if r != null { return r }
    r = parseStmt_enum(sess)
    if r != null { return r }
    r = parseStmt_class(sess)
    if r != null { return r }
    r = parseStmt_iface(sess)
    if r != null { return r }
    r = parseStmt_fn(sess)
    if r != null { return r }
    return null
}
""",
    )
    write(
        "parser_stmt_dispatch_ctrl.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_if"
import "self_host/parser_stmt_try"
import "self_host/parser_stmt_for"
import "self_host/parser_stmt_while"
import "self_host/parser_stmt_continue"
import "self_host/parser_stmt_break"
import "self_host/parser_stmt_throw"
import "self_host/parser_stmt_return"
import "self_host/parser_stmt_block"
import "self_host/parser_stmt_lateAssign"
import "self_host/parser_stmt_expr"

pub fn parseStmt_dispatch_ctrl(sess) {
    let r = parseStmt_if(sess)
    if r != null { return r }
    r = parseStmt_try(sess)
    if r != null { return r }
    r = parseStmt_for(sess)
    if r != null { return r }
    r = parseStmt_while(sess)
    if r != null { return r }
    r = parseStmt_continue(sess)
    if r != null { return r }
    r = parseStmt_break(sess)
    if r != null { return r }
    r = parseStmt_throw(sess)
    if r != null { return r }
    r = parseStmt_return(sess)
    if r != null { return r }
    r = parseStmt_block(sess)
    if r != null { return r }
    r = parseStmt_lateAssign(sess)
    if r != null { return r }
    return parseStmt_expr(sess)
}
""",
    )
    write(
        "parser_stmt.kab",
        IMPORTS
        + """
import "self_host/parser_stmt_dispatch_decl"
import "self_host/parser_stmt_dispatch_ctrl"

pub fn parseStmt(sess) {
    if sess["pCur"].type == "EOF" {
        return null
    }
    sess["pIsPub"] = 0
    if sess["pCur"].type == TOKEN_PUB {
        bump(sess)
        sess["pIsPub"] = 1
    }
    let r = parseStmt_dispatch_decl(sess)
    if r != null {
        return r
    }
    return parseStmt_dispatch_ctrl(sess)
}
""",
    )


def split_main_loop() -> None:
    if already("parser_main_loop.kab", "parser_main_loop_step"):
        print("skip parser_main_loop")
        return
    write(
        "parser_main_loop_step.kab",
        IMPORTS
        + """
pub fn parseMain_loop_step(sess) {
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
    return 0
}
""",
    )
    write(
        "parser_main_loop.kab",
        IMPORTS
        + """
import "self_host/parser_main_loop_step"

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
            parseMain_loop_step(sess)
        }
    }
    return 0
}
""",
    )


def split_tail_member() -> None:
    if already("parser_postfix_tail_member.kab", "parser_postfix_tail_member_name"):
        print("skip parser_postfix_tail_member")
        return
    write(
        "parser_postfix_tail_member_name.kab",
        IMPORTS
        + """
pub fn parsePostfix_tail_member_name(sess) {
    sess["pTok"] = sess["pCur"]
    sess["pMemberName"] = ""
    if sess["pTok"].type == TOKEN_IDENT {
        bump(sess)
        sess["pMemberName"] = sess["pTok"].value
        return 0
    }
    if sess["pTok"].type == TOKEN_NONE {
        bump(sess)
        sess["pMemberName"] = "None"
        return 0
    }
    if sess["pTok"].type == TOKEN_SOME {
        bump(sess)
        sess["pMemberName"] = "Some"
        return 0
    }
    if sess["pTok"].type == TOKEN_OK {
        bump(sess)
        sess["pMemberName"] = "Ok"
        return 0
    }
    if sess["pTok"].type == TOKEN_ERR {
        bump(sess)
        sess["pMemberName"] = "Err"
        return 0
    }
    throw "Expected member name"
}
""",
    )
    write(
        "parser_postfix_tail_member.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_tail_member_name"

pub fn parsePostfix_tail_member(sess) {
    if sess["pCur"].type != "." {
        return 0
    }
    bump(sess)
    parsePostfix_tail_member_name(sess)
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


def main() -> None:
    write_block_helpers()
    split_hooks()
    split_class_method()
    split_postfix_paren()
    split_iface_meth()
    split_obj_arr()
    split_fn_sig()
    split_logic_or()
    split_lit_scalar()
    rewrite_fn_body_and_arrows()
    split_stmt_dispatch()
    split_main_loop()
    split_tail_member()
    print("done — run test_parser.kab and _parser_all_shard_times.py")


if __name__ == "__main__":
    main()
