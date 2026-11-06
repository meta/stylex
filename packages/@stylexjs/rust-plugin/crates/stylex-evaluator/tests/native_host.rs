use std::{cell::RefCell,rc::Rc};
use indexmap::IndexMap;
use stylex_evaluator::evaluate;
use stylex_state::{functions::{FunctionMap,FunctionConfigType},state_manager::StateManager};
use stylex_structures::{stylex_env::*,stylex_options::StyleXOptions,named_import_source::ImportSources};
use swc_core::{common::{GLOBALS,Globals,FileName,SourceMap,DUMMY_SP},ecma::ast::{Ident,Expr,Lit}};
use swc_ecma_parser::{Parser,StringInput,Syntax,EsSyntax};
#[derive(Default)]
struct TestHost{calls:RefCell<Vec<(String,String)>>,gets:RefCell<Vec<String>>}
fn reference(id:&str,kind:HostKind)->HostRef{HostRef{owner:1,id:id.into(),kind}}
fn unused<T>()->HostResult<T>{Err(HostError{message:"Unexpected host operation in test".into()})}
impl HostEnv for TestHost{
 fn owner(&self)->u64{1}
 fn get(&self,obj:&HostRef,key:&str)->HostResult<HostValue>{self.gets.borrow_mut().push(key.into());assert_eq!(obj.id.as_ref(),"object");Ok(match key{"k"=>HostValue::Number(7.0),"add"=>HostValue::Ref(reference("add",HostKind::Function)),_=>HostValue::Undefined})}
 fn call(&self,f:&HostRef,this:&HostValue,args:&[HostValue])->HostResult<HostValue>{let HostValue::Ref(this)=this else{return unused();};self.calls.borrow_mut().push((f.id.to_string(),this.id.to_string()));if f.id.as_ref()=="count"{return Ok(HostValue::String(format!("x{}",self.calls.borrow().len())));}
 let Some(HostValue::Number(value))=args.first()else{return unused();};Ok(HostValue::Number(value+1.0))}
 fn same_value(&self,_a:&HostValue,_b:&HostValue)->HostResult<bool>{unused()}
 fn binary(&self,_op:&str,_a:&HostValue,_b:&HostValue)->HostResult<HostValue>{unused()}
 fn unary(&self,_op:&str,_a:&HostValue)->HostResult<HostValue>{unused()}
 fn to_primitive(&self,_v:&HostRef,_hint:PrimitiveHint)->HostResult<HostValue>{unused()}
 fn to_string(&self,_v:&HostValue)->HostResult<HostValue>{unused()}
 fn to_number(&self,_v:&HostValue)->HostResult<HostValue>{unused()}
 fn truthy(&self,_v:&HostValue)->HostResult<bool>{unused()}
 fn own_entries(&self,_v:&HostRef)->HostResult<Vec<(String,HostValue)>>{unused()}
 fn iterate(&self,_v:&HostRef)->HostResult<Vec<HostValue>>{unused()}
 fn is_array(&self,_v:&HostRef)->HostResult<bool>{unused()}
 fn static_builtin(&self,_ns:&str,_m:&str,_args:&[HostValue])->HostResult<HostValue>{unused()}
 fn materialize(&self,_v:&HostRef)->HostResult<Option<Materialized>>{unused()}
 fn register_function(&self,_f:HostFunction)->HostResult<HostValue>{unused()}
}
#[test]
fn live_host_entries_invoke_get_and_call_once_and_keep_receivers(){
 GLOBALS.set(&Globals::new(),||{
  for (source,expected,gets,this)in [("stylex.env.k",5.0,0,None),("stylex.env.add(2)",3.0,0,Some("env")),("stylex.env.object.k",7.0,1,None),("stylex.env.object.add(2)",3.0,1,Some("object"))]{
   let host=Rc::new(TestHost::default());let mut options=StyleXOptions::default();options.core.host_environment=Some(HostEnvironment{host:host.clone(),env:Some(reference("env",HostKind::PlainObject))});
   let mut traversal=StateManager::new(options);traversal.insert_stylex_namespace_binding(Ident::new("stylex".into(),DUMMY_SP,Default::default()).to_id());
   let mut env=IndexMap::new();env.insert("k".into(),EnvEntry::Host(HostValue::Number(5.0)));env.insert("add".into(),EnvEntry::Host(HostValue::Ref(reference("add",HostKind::Function))));env.insert("object".into(),EnvEntry::Host(HostValue::Ref(reference("object",HostKind::PlainObject))));
   let mut functions=FunctionMap::default();let mut entries=rustc_hash::FxHashMap::default();entries.insert("env".into(),Box::new(FunctionConfigType::EnvObject(Rc::new(env))));functions.member_expressions.insert(ImportSources::Regular("stylex".into()),Box::new(entries));
   let map=SourceMap::default();let file=map.new_source_file(FileName::Custom("host.js".into()).into(),source.to_owned());let mut parser=Parser::new(Syntax::Es(EsSyntax::default()),StringInput::from(&*file),None);let expr=match parser.parse_expr(){Ok(expr)=>expr,Err(error)=>panic!("{error:?}")};
   let result=evaluate::evaluate(&expr,&mut traversal,&functions);assert!(result.confident,"{source}: {:?}",result.reason);let Some(value)=result.value else{panic!("{source}: no value")};let Some(Expr::Lit(Lit::Num(number)))=value.as_expr()else{panic!("{source}: {value:?}")};assert_eq!(number.value,expected);assert_eq!(host.gets.borrow().len(),gets,"{source}");
   assert_eq!(host.calls.borrow().len(),usize::from(this.is_some()),"{source}");if let Some(this)=this{assert_eq!(host.calls.borrow()[0].1,this,"{source}");}
  }
 });
}
