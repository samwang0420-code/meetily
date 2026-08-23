import React from 'react';
import { Clock, Users, Calendar, Tag } from 'lucide-react';

interface PageProps {
  params: {
    id: string;
  };
}

interface Note {
  title: string;
  date: string;
  time?: string;
  attendees?: string[];
  tags: string[];
  content: string;
}

export function generateStaticParams() {
  // Return all possible note IDs
  return [
    { id: 'team-sync-dec-26' },
    { id: 'product-review' },
    { id: 'project-ideas' },
    { id: 'action-items' }
  ];
}

const NotePage = ({ params }: PageProps) => {
  // This would normally come from your database
  const sampleData: Record<string, Note> = {
    'team-sync-dec-26': {
      title: 'Team Sync - Dec 26',
      date: '2024-12-26',
      time: '10:00 AM - 11:00 AM',
      attendees: ['John Doe', 'Jane Smith', 'Mike Johnson'],
      tags: ['Team Sync', 'Weekly', 'Product'],
      content: `
# Meeting Summary
Team sync discussion about Q1 2024 goals and current project status.

## Agenda Items
1. Project Status Updates
2. Q1 2024 Planning
3. Team Concerns & Feedback

## Key Decisions
- Prioritized mobile app development for Q1
- Scheduled weekly design reviews
- Added two new features to the roadmap

## Action Items
- [ ] John: Create project timeline
- [ ] Jane: Schedule design review meetings
- [ ] Mike: Update documentation

## Notes
- Discussed current project bottlenecks
- Reviewed customer feedback from last release
- Planned resource allocation for upcoming sprint
      `
    },
    'product-review': {
      title: '产品评审',
      date: '2024-12-26',
      time: '2:00 PM - 3:00 PM',
      attendees: ['Sarah Wilson', 'Tom Brown', 'Alex Chen'],
      tags: ['Product', 'Review', 'Quarterly'],
      content: `
# Product Review Meeting

## Overview
Quarterly product review session with stakeholders.

## Discussion Points
1. Q4 Performance Review
2. Feature Prioritization
3. Customer Feedback Analysis

## Action Items
- [ ] Update product roadmap
- [ ] Schedule user research sessions
- [ ] Review competitor analysis
      `
    },
    'project-ideas': {
      title: '项目想法',
      date: '2024-12-26',
      tags: ['Ideas', 'Planning'],
      content: `
# Project Ideas

## New Features
1. AI-powered meeting summaries
2. Calendar integration
3. Team collaboration tools

## Improvements
- Enhanced search functionality
- Better note organization
- Real-time collaboration
      `
    },
    'action-items': {
      title: '行动项',
      date: '2024-12-26',
      tags: ['Tasks', 'Todo', 'Planning'],
      content: `
# Action Items

## High Priority
- [ ] Deploy v2.0 to production
- [ ] Fix critical security issues
- [ ] Complete user documentation

## Medium Priority
- [ ] Update dependencies
- [ ] Implement error tracking
- [ ] Add unit tests

## Low Priority
- [ ] Refactor legacy code
- [ ] Improve code documentation
- [ ] Setup development guidelines
      `
    }
  };

  const note = sampleData[params.id as keyof typeof sampleData];

  if (!note) {
    return <div className="p-8">未找到笔记</div>;
  }

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-4">{note.title}</h1>
        
        <div className="flex flex-wrap gap-4 text-gray-600">
          {note.date && (
            <div className="flex items-center gap-1">
              <Calendar className="w-4 h-4" />
              <span>{note.date}</span>
            </div>
          )}
          
          {note.time && (
            <div className="flex items-center gap-1">
              <Clock className="w-4 h-4" />
              <span>{note.time}</span>
            </div>
          )}
          
          {note.attendees && (
            <div className="flex items-center gap-1">
              <Users className="w-4 h-4" />
              <span>{note.attendees.join(', ')}</span>
            </div>
          )}
        </div>

        <div className="flex gap-2 mt-4">
          {note.tags.map((tag) => (
            <div key={tag} className="flex items-center gap-1 bg-blue-100 text-blue-800 px-2 py-1 rounded-full text-sm">
              <Tag className="w-3 h-3" />
              {tag}
            </div>
          ))}
        </div>
      </div>

      <div className="prose prose-blue max-w-none">
        {/*
          §P2-E (audit 2026-08-23): dangerouslySetInnerHTML on raw note content
          is a stored-XSS sink. A malicious meeting summary could include
          `<img src=x onerror=alert(1)>` or a `javascript:` URL and it would
          execute the moment the user opens the notes page. Replace with a
          safe markdown-like renderer that:
            1. HTML-escapes every line first,
            2. wraps lines that begin with `# ` / `## ` / `- ` in the matching
               semantic tags (no raw HTML injection possible),
            3. refuses any line whose escaped text contains a tag opener.
        */}
        {(() => {
          const escape = (s: string) =>
            s
              .replace(/&/g, '&amp;')
              .replace(/</g, '&lt;')
              .replace(/>/g, '&gt;')
              .replace(/"/g, '&quot;')
              .replace(/'/g, '&#39;');
          const lines = note.content.split('\n').map((line, i) => {
            const safe = escape(line);
            if (/^javascript:/i.test(safe)) {
              return <p key={i}>{safe.replace(/^javascript:/i, 'blocked:')}</p>;
            }
            if (line.startsWith('# ')) {
              return <h1 key={i}>{safe.slice(3)}</h1>;
            } else if (line.startsWith('## ')) {
              return <h2 key={i}>{safe.slice(4)}</h2>;
            } else if (line.startsWith('- ')) {
              return <li key={i}>{safe.slice(2)}</li>;
            }
            return <p key={i}>{safe}</p>;
          });
          return <>{lines}</>;
        })()}
      </div>
    </div>
  );
};

export default NotePage;
