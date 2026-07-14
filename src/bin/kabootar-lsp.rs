use kabootar_lib::language::{analyze, completions_at, goto_definition, hover_at, word_at_position, CompletionKind, Severity};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), "<".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "kabootar-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Kabootar LSP ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.documents.write().await.insert(uri.clone(), text.clone());
        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri.clone();
            let text = change.text.clone();
            self.documents.write().await.insert(uri.clone(), text.clone());
            self.publish_diagnostics(&uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.write().await.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let doc = self.documents.read().await.get(&uri).cloned().unwrap_or_default();
        let pos = params.text_document_position.position;
        let prefix = word_at_position(pos.line, pos.character, &doc);
        let items: Vec<CompletionItem> = completions_at(&doc, pos.line, pos.character, &prefix)
            .into_iter()
            .map(|item| CompletionItem {
                label: item.label,
                detail: item.detail,
                kind: Some(match item.kind {
                    CompletionKind::Keyword => CompletionItemKind::KEYWORD,
                    CompletionKind::Function => CompletionItemKind::FUNCTION,
                    CompletionKind::Module => CompletionItemKind::MODULE,
                    CompletionKind::Type => CompletionItemKind::TYPE_PARAMETER,
                    CompletionKind::Generic => CompletionItemKind::CLASS,
                }),
                ..Default::default()
            })
            .collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let doc = self.documents.read().await.get(&uri).cloned().unwrap_or_default();
        let pos = params.text_document_position_params.position;
        Ok(hover_at(&doc, pos.line, pos.character).map(|contents| Hover {
            contents: HoverContents::Scalar(MarkedString::String(contents)),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let Some(doc) = self.documents.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let Some(target) = goto_definition(&doc, pos.line, pos.character) else {
            return Ok(None);
        };

        let target_uri = if let Some(module) = target.module {
            Url::parse(&format!("kabootar://module/{}", module)).unwrap_or(uri.clone())
        } else {
            uri
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range: kabootar_span_to_range(target.line, target.column, target.len),
        })))
    }
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn publish_diagnostics(&self, uri: &Url, text: &str) {
        let diagnostics: Vec<Diagnostic> = analyze(text)
            .into_iter()
            .map(|d| Diagnostic {
                range: Range {
                    start: Position {
                        line: d.line.saturating_sub(1),
                        character: d.column.saturating_sub(1),
                    },
                    end: Position {
                        line: d.line.saturating_sub(1),
                        character: d.column.saturating_sub(1) + d.len,
                    },
                },
                severity: Some(match d.severity {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                }),
                message: d.message,
                source: Some("kabootar".into()),
                ..Default::default()
            })
            .collect();

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn kabootar_span_to_range(line: u32, column: u32, len: u32) -> Range {
    Range {
        start: Position {
            line: line.saturating_sub(1),
            character: column.saturating_sub(1),
        },
        end: Position {
            line: line.saturating_sub(1),
            character: column.saturating_sub(1) + len,
        },
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
