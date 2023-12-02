"use strict";(self.webpackChunkthalo_docs=self.webpackChunkthalo_docs||[]).push([[288],{3905:(e,t,n)=>{n.d(t,{Zo:()=>d,kt:()=>v});var r=n(7294);function a(e,t,n){return t in e?Object.defineProperty(e,t,{value:n,enumerable:!0,configurable:!0,writable:!0}):e[t]=n,e}function o(e,t){var n=Object.keys(e);if(Object.getOwnPropertySymbols){var r=Object.getOwnPropertySymbols(e);t&&(r=r.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),n.push.apply(n,r)}return n}function i(e){for(var t=1;t<arguments.length;t++){var n=null!=arguments[t]?arguments[t]:{};t%2?o(Object(n),!0).forEach((function(t){a(e,t,n[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(n)):o(Object(n)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(n,t))}))}return e}function s(e,t){if(null==e)return{};var n,r,a=function(e,t){if(null==e)return{};var n,r,a={},o=Object.keys(e);for(r=0;r<o.length;r++)n=o[r],t.indexOf(n)>=0||(a[n]=e[n]);return a}(e,t);if(Object.getOwnPropertySymbols){var o=Object.getOwnPropertySymbols(e);for(r=0;r<o.length;r++)n=o[r],t.indexOf(n)>=0||Object.prototype.propertyIsEnumerable.call(e,n)&&(a[n]=e[n])}return a}var l=r.createContext({}),c=function(e){var t=r.useContext(l),n=t;return e&&(n="function"==typeof e?e(t):i(i({},t),e)),n},d=function(e){var t=c(e.components);return r.createElement(l.Provider,{value:t},e.children)},u="mdxType",m={inlineCode:"code",wrapper:function(e){var t=e.children;return r.createElement(r.Fragment,{},t)}},p=r.forwardRef((function(e,t){var n=e.components,a=e.mdxType,o=e.originalType,l=e.parentName,d=s(e,["components","mdxType","originalType","parentName"]),u=c(n),p=a,v=u["".concat(l,".").concat(p)]||u[p]||m[p]||o;return n?r.createElement(v,i(i({ref:t},d),{},{components:n})):r.createElement(v,i({ref:t},d))}));function v(e,t){var n=arguments,a=t&&t.mdxType;if("string"==typeof e||a){var o=n.length,i=new Array(o);i[0]=p;var s={};for(var l in t)hasOwnProperty.call(t,l)&&(s[l]=t[l]);s.originalType=e,s[u]="string"==typeof e?e:a,i[1]=s;for(var c=2;c<o;c++)i[c]=n[c];return r.createElement.apply(null,i)}return r.createElement.apply(null,n)}p.displayName="MDXCreateElement"},8245:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>l,contentTitle:()=>i,default:()=>u,frontMatter:()=>o,metadata:()=>s,toc:()=>c});var r=n(7462),a=(n(7294),n(3905));const o={sidebar_position:5},i="Event Store Management",s={unversionedId:"event-store-management",id:"event-store-management",title:"Event Store Management",description:"In Thalo, the management of events is a process handled efficiently and seamlessly by the runtime, using Sled as the embedded event store. This section provides an overview of this automated event management system.",source:"@site/docs/event-store-management.md",sourceDirName:".",slug:"/event-store-management",permalink:"/thalo/docs/event-store-management",draft:!1,editUrl:"https://github.com/thalo-rs/thalo-docs/tree/main/packages/create-docusaurus/templates/shared/docs/event-store-management.md",tags:[],version:"current",sidebarPosition:5,frontMatter:{sidebar_position:5},sidebar:"tutorialSidebar",previous:{title:"Writing and Deploying Aggregates",permalink:"/thalo/docs/writing-and-deploying-aggregates"},next:{title:"Projections and Event Handling",permalink:"/thalo/docs/projections-and-event-handling"}},l={},c=[{value:"Automated Event Persistence",id:"automated-event-persistence",level:2},{value:"Efficient Event Retrieval",id:"efficient-event-retrieval",level:2}],d={toc:c};function u(e){let{components:t,...n}=e;return(0,a.kt)("wrapper",(0,r.Z)({},d,n,{components:t,mdxType:"MDXLayout"}),(0,a.kt)("h1",{id:"event-store-management"},"Event Store Management"),(0,a.kt)("p",null,"In Thalo, the management of events is a process handled efficiently and seamlessly by the runtime, using Sled as the embedded event store. This section provides an overview of this automated event management system."),(0,a.kt)("h2",{id:"automated-event-persistence"},"Automated Event Persistence"),(0,a.kt)("p",null,"Thalo's runtime automates the persistence of events generated by aggregates. This involves storing each event in the Sled database as it occurs, ensuring data integrity and reliability."),(0,a.kt)("h2",{id:"efficient-event-retrieval"},"Efficient Event Retrieval"),(0,a.kt)("p",null,"The runtime also manages the retrieval of events. When an aggregate state needs to be rebuilt, Thalo efficiently fetches the relevant events from the store, reconstructing the aggregate's state without developer intervention."),(0,a.kt)("p",null,"Thalo uses LRU cache to store the state of recently accessed aggregates without having to rebuild them each time.\nThis can be configured with the ",(0,a.kt)("inlineCode",{parentName:"p"},"--cache-size")," flag in the thalo runtime."))}u.isMDXComponent=!0}}]);